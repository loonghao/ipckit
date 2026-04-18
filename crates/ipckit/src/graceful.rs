//! Graceful shutdown mechanism for IPC channels
//!
//! This module provides the `GracefulChannel` trait for implementing graceful shutdown
//! in IPC channels, preventing errors like `EventLoopClosed` when background threads
//! continue sending messages after the main event loop has closed.
//!
//! # Reentrancy-safe submit
//!
//! [`GracefulIpcChannel`] also provides [`GracefulIpcChannel::submit_reentrant`], which
//! avoids the deadlock that occurs when a task running on an affinity-pinned thread
//! (e.g. the DCC main thread) tries to dispatch a follow-up task back to the same thread
//! and block on its result.  The call sequence mirrors Swift's `MainActor.assumeIsolated`
//! and C#'s `SynchronizationContext.Send`:
//!
//! - If the calling thread **is** the target affinity thread → execute `f` inline.
//! - Otherwise → push `f` into the dispatch queue and block until the affinity thread
//!   picks it up via [`GracefulIpcChannel::pump_pending`].
//!
//! ```rust,no_run
//! use ipckit::{GracefulIpcChannel, IpcError};
//!
//! let channel = GracefulIpcChannel::<Vec<u8>>::create("my_channel")?;
//! channel.bind_affinity_thread();        // Called once, on the "main" thread
//!
//! // On a worker thread:
//! channel.submit_reentrant(|| {
//!     // Runs inline when called from the main thread,
//!     // or is queued + awaited from any other thread.
//!     println!("Hello from the affinity thread!");
//! })?;
//!
//! // In the main-thread idle callback:
//! channel.pump_pending(std::time::Duration::from_millis(10));
//! # Ok::<(), IpcError>(())
//! ```
//!
//! # Example
//!
//! ```rust,no_run
//! use ipckit::{NamedPipe, GracefulChannel, GracefulNamedPipe};
//! use std::time::Duration;
//!
//! fn main() -> Result<(), ipckit::IpcError> {
//!     let pipe = NamedPipe::create("my_pipe")?;
//!     let graceful = GracefulNamedPipe::new(pipe);
//!
//!     // ... use the channel ...
//!
//!     // Graceful shutdown
//!     graceful.shutdown();
//!     graceful.drain()?; // Wait for pending messages
//!
//!     Ok(())
//! }
//! ```

use crate::error::{IpcError, Result};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Trait for channels that support graceful shutdown
///
/// This trait provides methods for signaling shutdown, checking shutdown status,
/// and draining pending messages before closing the channel.
pub trait GracefulChannel {
    /// Signal the channel to shutdown
    ///
    /// After calling this method:
    /// - New send operations will return `IpcError::Closed`
    /// - Pending messages may still be processed (use `drain()` to wait)
    /// - `is_shutdown()` will return `true`
    fn shutdown(&self);

    /// Check if the channel has been signaled to shutdown
    fn is_shutdown(&self) -> bool;

    /// Wait for all pending messages to be processed
    ///
    /// This method blocks until all messages that were in-flight before
    /// `shutdown()` was called have been processed.
    fn drain(&self) -> Result<()>;

    /// Shutdown with a timeout
    ///
    /// Combines `shutdown()` and `drain()` with a timeout.
    /// Returns `IpcError::Timeout` if the drain doesn't complete within the timeout.
    fn shutdown_timeout(&self, timeout: Duration) -> Result<()>;
}

/// Shutdown state that can be shared between channel instances
#[derive(Debug)]
pub struct ShutdownState {
    /// Whether shutdown has been signaled
    shutdown: AtomicBool,
    /// Number of pending operations
    pending_count: AtomicUsize,
}

impl Default for ShutdownState {
    fn default() -> Self {
        Self::new()
    }
}

impl ShutdownState {
    /// Create a new shutdown state
    pub fn new() -> Self {
        Self {
            shutdown: AtomicBool::new(false),
            pending_count: AtomicUsize::new(0),
        }
    }

    /// Signal shutdown
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }

    /// Check if shutdown has been signaled
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    /// Increment pending operation count
    pub fn begin_operation(&self) -> Result<OperationGuard<'_>> {
        if self.is_shutdown() {
            return Err(IpcError::Closed);
        }
        self.pending_count.fetch_add(1, Ordering::SeqCst);

        // Double-check after incrementing to prevent race condition
        if self.is_shutdown() {
            self.pending_count.fetch_sub(1, Ordering::SeqCst);
            return Err(IpcError::Closed);
        }

        Ok(OperationGuard { state: self })
    }

    /// Get the current pending operation count
    pub fn pending_count(&self) -> usize {
        self.pending_count.load(Ordering::SeqCst)
    }

    /// Wait for all pending operations to complete
    pub fn wait_for_drain(&self, timeout: Option<Duration>) -> Result<()> {
        let start = Instant::now();
        let sleep_duration = Duration::from_millis(1);

        loop {
            if self.pending_count() == 0 {
                return Ok(());
            }

            if let Some(timeout) = timeout {
                if start.elapsed() >= timeout {
                    return Err(IpcError::Timeout);
                }
            }

            std::thread::sleep(sleep_duration);
        }
    }
}

/// RAII guard for tracking pending operations
pub struct OperationGuard<'a> {
    state: &'a ShutdownState,
}

impl Drop for OperationGuard<'_> {
    fn drop(&mut self) {
        self.state.pending_count.fetch_sub(1, Ordering::SeqCst);
    }
}

/// A wrapper that adds graceful shutdown capability to any channel
#[derive(Debug)]
pub struct GracefulWrapper<T> {
    inner: T,
    state: Arc<ShutdownState>,
}

impl<T> GracefulWrapper<T> {
    /// Create a new graceful wrapper around a channel
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            state: Arc::new(ShutdownState::new()),
        }
    }

    /// Create a new graceful wrapper with a shared shutdown state
    pub fn with_state(inner: T, state: Arc<ShutdownState>) -> Self {
        Self { inner, state }
    }

    /// Get a reference to the inner channel
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Get a mutable reference to the inner channel
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Get the shutdown state
    pub fn state(&self) -> Arc<ShutdownState> {
        Arc::clone(&self.state)
    }

    /// Consume the wrapper and return the inner channel
    pub fn into_inner(self) -> T {
        self.inner
    }

    /// Begin an operation, returning a guard that tracks the operation
    pub fn begin_operation(&self) -> Result<OperationGuard<'_>> {
        self.state.begin_operation()
    }
}

impl<T> GracefulChannel for GracefulWrapper<T> {
    fn shutdown(&self) {
        self.state.shutdown();
    }

    fn is_shutdown(&self) -> bool {
        self.state.is_shutdown()
    }

    fn drain(&self) -> Result<()> {
        self.state.wait_for_drain(None)
    }

    fn shutdown_timeout(&self, timeout: Duration) -> Result<()> {
        self.shutdown();
        self.state.wait_for_drain(Some(timeout))
    }
}

impl<T: Clone> Clone for GracefulWrapper<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            state: Arc::clone(&self.state),
        }
    }
}

// ============================================================================
// GracefulNamedPipe - Named pipe with graceful shutdown
// ============================================================================

use crate::pipe::NamedPipe;
use std::io::{Read, Write};

/// Named pipe with graceful shutdown support
pub struct GracefulNamedPipe {
    inner: NamedPipe,
    state: Arc<ShutdownState>,
}

impl GracefulNamedPipe {
    /// Create a new graceful named pipe wrapper
    pub fn new(pipe: NamedPipe) -> Self {
        Self {
            inner: pipe,
            state: Arc::new(ShutdownState::new()),
        }
    }

    /// Create a new graceful named pipe with a shared shutdown state
    pub fn with_state(pipe: NamedPipe, state: Arc<ShutdownState>) -> Self {
        Self { inner: pipe, state }
    }

    /// Create a new named pipe server with graceful shutdown
    pub fn create(name: &str) -> Result<Self> {
        let pipe = NamedPipe::create(name)?;
        Ok(Self::new(pipe))
    }

    /// Connect to an existing named pipe with graceful shutdown
    pub fn connect(name: &str) -> Result<Self> {
        let pipe = NamedPipe::connect(name)?;
        Ok(Self::new(pipe))
    }

    /// Get the pipe name
    pub fn name(&self) -> &str {
        self.inner.name()
    }

    /// Check if this is the server end
    pub fn is_server(&self) -> bool {
        self.inner.is_server()
    }

    /// Wait for a client to connect (server only)
    pub fn wait_for_client(&mut self) -> Result<()> {
        if self.state.is_shutdown() {
            return Err(IpcError::Closed);
        }
        self.inner.wait_for_client()
    }

    /// Get the shutdown state for sharing with other channels
    pub fn state(&self) -> Arc<ShutdownState> {
        Arc::clone(&self.state)
    }

    /// Get a reference to the inner pipe
    pub fn inner(&self) -> &NamedPipe {
        &self.inner
    }

    /// Get a mutable reference to the inner pipe
    pub fn inner_mut(&mut self) -> &mut NamedPipe {
        &mut self.inner
    }
}

impl GracefulChannel for GracefulNamedPipe {
    fn shutdown(&self) {
        self.state.shutdown();
    }

    fn is_shutdown(&self) -> bool {
        self.state.is_shutdown()
    }

    fn drain(&self) -> Result<()> {
        self.state.wait_for_drain(None)
    }

    fn shutdown_timeout(&self, timeout: Duration) -> Result<()> {
        self.shutdown();
        self.state.wait_for_drain(Some(timeout))
    }
}

impl Read for GracefulNamedPipe {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.state.is_shutdown() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "Channel is shutdown",
            ));
        }

        let _guard = self.state.begin_operation().map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Channel is shutdown")
        })?;

        self.inner.read(buf)
    }
}

impl Write for GracefulNamedPipe {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.state.is_shutdown() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "Channel is shutdown",
            ));
        }

        let _guard = self.state.begin_operation().map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Channel is shutdown")
        })?;

        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

// ============================================================================
// ReentrantDispatch – thread-affinity + reentrancy-safe submit
// ============================================================================

use crossbeam_channel as cb;
use std::thread::ThreadId;

/// The result of a [`ReentrantDispatch::submit_reentrant`] call.
///
/// Callers only care about whether the function ran successfully; the concrete
/// return type is erased to keep the queue homogeneous.
type BoxResult = std::result::Result<(), Box<dyn std::error::Error + Send + Sync>>;

/// A work item pushed into the cross-thread queue.
struct WorkItem {
    /// The closure to execute on the affinity thread.
    func: Box<dyn FnOnce() -> BoxResult + Send>,
    /// One-shot channel for sending the result back to the waiter.
    reply: cb::Sender<BoxResult>,
}

/// Internal state shared between all clones of a channel.
struct DispatchState {
    /// Thread ID of the bound "affinity" thread, if any.
    affinity_thread: parking_lot::RwLock<Option<ThreadId>>,
    /// Cross-thread work queue.
    tx: cb::Sender<WorkItem>,
    /// Number of items still pending in the queue (for diagnostics).
    pending: AtomicUsize,
}

/// Reentrancy-safe dispatcher for affinity-pinned threads.
///
/// Attach one of these to `GracefulIpcChannel` so that callers can safely
/// dispatch work back to the "main" (or any named) thread without deadlocking
/// when the caller itself *is* the affinity thread.
///
/// The pattern mirrors Swift's `MainActor.assumeIsolated` and C#'s
/// `SynchronizationContext.Send`:
///
/// - **Caller is affinity thread** → execute `f` inline, return immediately.
/// - **Caller is any other thread** → push `f` to the queue, block until the
///   affinity thread drains it via [`ReentrantDispatch::pump`].
#[derive(Clone)]
pub struct ReentrantDispatch {
    state: Arc<DispatchState>,
    rx: Arc<cb::Receiver<WorkItem>>,
}

impl ReentrantDispatch {
    /// Create a new dispatch queue.
    pub fn new() -> Self {
        let (tx, rx) = cb::unbounded();
        Self {
            state: Arc::new(DispatchState {
                affinity_thread: parking_lot::RwLock::new(None),
                tx,
                pending: AtomicUsize::new(0),
            }),
            rx: Arc::new(rx),
        }
    }

    /// Bind the current thread as the affinity thread.
    ///
    /// Must be called **once** from the thread that will handle dispatched
    /// work (e.g. the DCC idle callback, Unity `EditorApplication.update`,
    /// Unreal `FTSTicker`, etc.).
    ///
    /// Calling this again from a different thread replaces the binding.
    pub fn bind_current_thread(&self) {
        *self.state.affinity_thread.write() = Some(std::thread::current().id());
    }

    /// Returns `true` if the calling thread is the bound affinity thread.
    pub fn is_affinity_thread(&self) -> bool {
        self.state
            .affinity_thread
            .read()
            .map_or(false, |id| id == std::thread::current().id())
    }

    /// Submit `f` to the affinity thread in a reentrancy-safe way.
    ///
    /// - If the current thread **is** the affinity thread, `f` is called
    ///   inline and its return value is propagated immediately — no queue,
    ///   no blocking.
    /// - Otherwise, `f` is pushed into the work queue and the calling thread
    ///   blocks until the affinity thread processes it via [`pump`](Self::pump).
    ///
    /// # Errors
    ///
    /// Returns `Err(IpcError::Closed)` if the affinity thread has been
    /// dropped (the receiver end of the queue is gone).
    ///
    /// Returns `Err(IpcError::Other(...))` if `f` itself returns an error.
    pub fn submit_reentrant<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        // ── Short-circuit: inline execution on the affinity thread ──────────
        if self.is_affinity_thread() {
            return Ok(f());
        }

        // ── Cross-thread dispatch ────────────────────────────────────────────
        let (reply_tx, reply_rx) = cb::bounded::<BoxResult>(1);

        // Wrap the return value so we can send it through the boxed channel.
        // We smuggle the `R` value via a `Mutex<Option<R>>` placed on the
        // heap so the closure stays `FnOnce() -> BoxResult`.
        let slot: Arc<parking_lot::Mutex<Option<R>>> = Arc::new(parking_lot::Mutex::new(None));
        let slot_write = Arc::clone(&slot);

        let item = WorkItem {
            func: Box::new(move || {
                let result = f();
                *slot_write.lock() = Some(result);
                Ok(())
            }),
            reply: reply_tx,
        };

        self.state.pending.fetch_add(1, Ordering::Relaxed);
        self.state
            .tx
            .send(item)
            .map_err(|_| IpcError::Closed)?;

        // Block until the affinity thread acknowledges.
        reply_rx
            .recv()
            .map_err(|_| IpcError::Closed)?
            .map_err(|e| IpcError::Other(e.to_string()))?;

        // Retrieve the return value produced by the closure.
        let value = slot
            .lock()
            .take()
            .expect("affinity thread must have set the value");
        Ok(value)
    }

    /// Drain at most one "budget" worth of pending work items on the current
    /// (affinity) thread.
    ///
    /// Call this from your host's idle callback:
    ///
    /// ```rust,no_run
    /// # use ipckit::ReentrantDispatch;
    /// # let dispatch = ReentrantDispatch::new();
    /// # dispatch.bind_current_thread();
    /// // Maya scriptJob idleEvent / Unity EditorApplication.update / …
    /// dispatch.pump(std::time::Duration::from_millis(8));
    /// ```
    ///
    /// Returns the number of items processed.
    pub fn pump(&self, budget: Duration) -> usize {
        let start = Instant::now();
        let mut count = 0;

        loop {
            if start.elapsed() >= budget {
                break;
            }

            match self.rx.try_recv() {
                Ok(item) => {
                    self.state.pending.fetch_sub(1, Ordering::Relaxed);
                    let result = (item.func)();
                    let _ = item.reply.send(result);
                    count += 1;
                }
                Err(cb::TryRecvError::Empty) => break,
                Err(cb::TryRecvError::Disconnected) => break,
            }
        }

        count
    }

    /// Number of items currently waiting in the queue.
    pub fn pending_count(&self) -> usize {
        self.state.pending.load(Ordering::Relaxed)
    }
}

impl Default for ReentrantDispatch {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// GracefulIpcChannel - IPC channel with graceful shutdown
// ============================================================================

use crate::channel::IpcChannel;
use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;

/// IPC channel with graceful shutdown support
pub struct GracefulIpcChannel<T = Vec<u8>> {
    inner: IpcChannel<T>,
    state: Arc<ShutdownState>,
    dispatch: ReentrantDispatch,
    _marker: PhantomData<T>,
}

impl<T> GracefulIpcChannel<T> {
    /// Create a new graceful IPC channel wrapper
    pub fn new(channel: IpcChannel<T>) -> Self {
        Self {
            inner: channel,
            state: Arc::new(ShutdownState::new()),
            dispatch: ReentrantDispatch::new(),
            _marker: PhantomData,
        }
    }

    /// Create a new graceful IPC channel with a shared shutdown state
    pub fn with_state(channel: IpcChannel<T>, state: Arc<ShutdownState>) -> Self {
        Self {
            inner: channel,
            state,
            dispatch: ReentrantDispatch::new(),
            _marker: PhantomData,
        }
    }

    /// Create a new IPC channel server with graceful shutdown
    pub fn create(name: &str) -> Result<Self> {
        let channel = IpcChannel::create(name)?;
        Ok(Self::new(channel))
    }

    /// Connect to an existing IPC channel with graceful shutdown
    pub fn connect(name: &str) -> Result<Self> {
        let channel = IpcChannel::connect(name)?;
        Ok(Self::new(channel))
    }

    /// Get the channel name
    pub fn name(&self) -> &str {
        self.inner.name()
    }

    /// Check if this is the server end
    pub fn is_server(&self) -> bool {
        self.inner.is_server()
    }

    /// Wait for a client to connect (server only)
    pub fn wait_for_client(&mut self) -> Result<()> {
        if self.state.is_shutdown() {
            return Err(IpcError::Closed);
        }
        self.inner.wait_for_client()
    }

    /// Get the shutdown state for sharing with other channels
    pub fn state(&self) -> Arc<ShutdownState> {
        Arc::clone(&self.state)
    }

    /// Get a reference to the inner channel
    pub fn inner(&self) -> &IpcChannel<T> {
        &self.inner
    }

    /// Get a mutable reference to the inner channel
    pub fn inner_mut(&mut self) -> &mut IpcChannel<T> {
        &mut self.inner
    }

    // ── Reentrancy-safe dispatch API ──────────────────────────────────────────

    /// Bind the current thread as the affinity thread for this channel's
    /// reentrancy-safe dispatch queue.
    ///
    /// Call this **once** on the "main" (or named) thread that will drive
    /// [`pump_pending`](Self::pump_pending).
    pub fn bind_affinity_thread(&self) {
        self.dispatch.bind_current_thread();
    }

    /// Submit `f` to run on the bound affinity thread in a deadlock-free way.
    ///
    /// - **Caller is the affinity thread** → `f` executes inline.
    /// - **Any other thread** → `f` is queued; the caller blocks until
    ///   [`pump_pending`](Self::pump_pending) processes it.
    ///
    /// Parallel to Swift's `MainActor.assumeIsolated` and C#'s
    /// `SynchronizationContext.Send`.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError::Closed`] if the channel has been shut down or the
    /// affinity thread has been dropped.
    pub fn submit_reentrant<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        if self.state.is_shutdown() {
            return Err(IpcError::Closed);
        }
        self.dispatch.submit_reentrant(f)
    }

    /// Drain at most `budget` of pending work items on the **current** thread.
    ///
    /// Call this from your host's idle callback (Maya `scriptJob idleEvent`,
    /// Unity `EditorApplication.update`, Unreal `FTSTicker`, Blender
    /// `bpy.app.timers`, etc.) to let cross-thread submissions complete.
    ///
    /// Returns the number of items processed.
    pub fn pump_pending(&self, budget: Duration) -> usize {
        self.dispatch.pump(budget)
    }
}

impl<T> GracefulChannel for GracefulIpcChannel<T> {
    fn shutdown(&self) {
        self.state.shutdown();
    }

    fn is_shutdown(&self) -> bool {
        self.state.is_shutdown()
    }

    fn drain(&self) -> Result<()> {
        self.state.wait_for_drain(None)
    }

    fn shutdown_timeout(&self, timeout: Duration) -> Result<()> {
        self.shutdown();
        self.state.wait_for_drain(Some(timeout))
    }
}

impl GracefulIpcChannel<Vec<u8>> {
    /// Send raw bytes
    pub fn send_bytes(&mut self, data: &[u8]) -> Result<()> {
        if self.state.is_shutdown() {
            return Err(IpcError::Closed);
        }

        let _guard = self.state.begin_operation()?;
        self.inner.send_bytes(data)
    }

    /// Receive raw bytes
    pub fn recv_bytes(&mut self) -> Result<Vec<u8>> {
        if self.state.is_shutdown() {
            return Err(IpcError::Closed);
        }

        let _guard = self.state.begin_operation()?;
        self.inner.recv_bytes()
    }
}

impl<T: Serialize + DeserializeOwned> GracefulIpcChannel<T> {
    /// Send a typed message (serialized as JSON)
    pub fn send(&mut self, msg: &T) -> Result<()> {
        if self.state.is_shutdown() {
            return Err(IpcError::Closed);
        }

        let _guard = self.state.begin_operation()?;
        self.inner.send(msg)
    }

    /// Receive a typed message (deserialized from JSON)
    pub fn recv(&mut self) -> Result<T> {
        if self.state.is_shutdown() {
            return Err(IpcError::Closed);
        }

        let _guard = self.state.begin_operation()?;
        self.inner.recv()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_shutdown_state() {
        let state = ShutdownState::new();

        assert!(!state.is_shutdown());
        assert_eq!(state.pending_count(), 0);

        state.shutdown();

        assert!(state.is_shutdown());
    }

    #[test]
    fn test_operation_guard() {
        let state = ShutdownState::new();

        {
            let _guard = state.begin_operation().unwrap();
            assert_eq!(state.pending_count(), 1);

            {
                let _guard2 = state.begin_operation().unwrap();
                assert_eq!(state.pending_count(), 2);
            }

            assert_eq!(state.pending_count(), 1);
        }

        assert_eq!(state.pending_count(), 0);
    }

    #[test]
    fn test_operation_after_shutdown() {
        let state = ShutdownState::new();

        state.shutdown();

        let result = state.begin_operation();
        assert!(result.is_err());
    }

    #[test]
    fn test_drain() {
        let state = Arc::new(ShutdownState::new());
        let state_clone = Arc::clone(&state);

        // Start a background operation
        let handle = thread::spawn(move || {
            let _guard = state_clone.begin_operation().unwrap();
            thread::sleep(Duration::from_millis(50));
        });

        // Give the thread time to start
        thread::sleep(Duration::from_millis(10));

        // Shutdown and drain
        state.shutdown();
        let result = state.wait_for_drain(Some(Duration::from_secs(1)));

        handle.join().unwrap();
        assert!(result.is_ok());
    }

    #[test]
    fn test_drain_timeout() {
        let state = Arc::new(ShutdownState::new());
        let state_clone = Arc::clone(&state);

        // Start a long background operation
        let handle = thread::spawn(move || {
            let _guard = state_clone.begin_operation().unwrap();
            thread::sleep(Duration::from_secs(10));
        });

        // Give the thread time to start
        thread::sleep(Duration::from_millis(10));

        // Shutdown with short timeout
        state.shutdown();
        let result = state.wait_for_drain(Some(Duration::from_millis(50)));

        assert!(matches!(result, Err(IpcError::Timeout)));

        // Clean up - we need to wait for the thread
        drop(state);
        // The thread will eventually finish
        let _ = handle.join();
    }

    #[test]
    fn test_graceful_wrapper() {
        let wrapper = GracefulWrapper::new(42);

        assert!(!wrapper.is_shutdown());
        assert_eq!(*wrapper.inner(), 42);

        wrapper.shutdown();

        assert!(wrapper.is_shutdown());
    }

    #[test]
    fn test_graceful_named_pipe() {
        let name = format!("test_graceful_pipe_{}", std::process::id());

        let handle = thread::spawn({
            let name = name.clone();
            move || {
                let mut server = GracefulNamedPipe::create(&name).unwrap();
                server.wait_for_client().ok();

                let mut buf = [0u8; 32];
                let n = server.read(&mut buf).unwrap();
                assert_eq!(&buf[..n], b"Hello!");

                // Shutdown
                server.shutdown();
                assert!(server.is_shutdown());

                // Operations after shutdown should fail
                let result = server.write(b"test");
                assert!(result.is_err());
            }
        });

        thread::sleep(Duration::from_millis(100));

        let mut client = GracefulNamedPipe::connect(&name).unwrap();
        client.write_all(b"Hello!").unwrap();

        handle.join().unwrap();
    }

    #[test]
    fn test_graceful_ipc_channel() {
        let name = format!("test_graceful_channel_{}", std::process::id());

        let handle = thread::spawn({
            let name = name.clone();
            move || {
                let mut server = GracefulIpcChannel::<Vec<u8>>::create(&name).unwrap();
                server.wait_for_client().ok();

                let data = server.recv_bytes().unwrap();
                assert_eq!(data, b"Hello, IPC!");

                // Shutdown
                server.shutdown();
                assert!(server.is_shutdown());

                // Operations after shutdown should fail
                let result = server.recv_bytes();
                assert!(matches!(result, Err(IpcError::Closed)));
            }
        });

        thread::sleep(Duration::from_millis(100));

        let mut client = GracefulIpcChannel::<Vec<u8>>::connect(&name).unwrap();
        client.send_bytes(b"Hello, IPC!").unwrap();

        handle.join().unwrap();
    }

    // ────────────────────────────────────────────────────────────────────────
    // ReentrantDispatch tests
    // ────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_reentrant_dispatch_inline_on_affinity_thread() {
        let dispatch = ReentrantDispatch::new();
        dispatch.bind_current_thread();

        // Called from the affinity thread → should execute inline (no queue).
        let result: i32 = dispatch.submit_reentrant(|| 42).unwrap();
        assert_eq!(result, 42);

        // Nothing should be pending.
        assert_eq!(dispatch.pending_count(), 0);
    }

    #[test]
    fn test_reentrant_dispatch_cross_thread() {
        let dispatch = ReentrantDispatch::new();
        dispatch.bind_current_thread(); // current thread = affinity

        let dispatch_worker = dispatch.clone();
        let dispatch_pump = dispatch.clone();

        // Spawn a worker that submits a closure to the affinity thread.
        let handle = thread::spawn(move || {
            // Worker submits; should block until pumped.
            let result: u64 = dispatch_worker
                .submit_reentrant(|| 99_u64)
                .expect("submit failed");
            assert_eq!(result, 99);
        });

        // Give the worker thread a moment to enqueue.
        thread::sleep(Duration::from_millis(20));

        // Pump on the affinity thread; must process the pending item.
        let processed = dispatch_pump.pump(Duration::from_millis(100));
        assert_eq!(processed, 1);

        handle.join().unwrap();
    }

    #[test]
    fn test_reentrant_dispatch_multiple_submissions() {
        let dispatch = ReentrantDispatch::new();
        dispatch.bind_current_thread();

        let counter = Arc::new(std::sync::atomic::AtomicU32::new(0));

        let handles: Vec<_> = (0..5)
            .map(|_| {
                let d = dispatch.clone();
                let c = Arc::clone(&counter);
                thread::spawn(move || {
                    d.submit_reentrant(move || {
                        c.fetch_add(1, Ordering::SeqCst);
                    })
                    .unwrap();
                })
            })
            .collect();

        // Give workers time to queue their items.
        thread::sleep(Duration::from_millis(30));

        // Drain with a generous budget.
        let processed = dispatch.pump(Duration::from_millis(500));
        assert_eq!(processed, 5);

        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(counter.load(Ordering::SeqCst), 5);
    }

    #[test]
    fn test_graceful_channel_submit_reentrant() {
        let name = format!("test_reentrant_channel_{}", std::process::id());

        // Create the server channel and bind the current thread as affinity.
        let server = GracefulIpcChannel::<Vec<u8>>::create(&name).unwrap();
        server.bind_affinity_thread();

        // Called from the affinity thread → inline execution.
        let result: &'static str = server
            .submit_reentrant(|| "hello from affinity")
            .unwrap();
        assert_eq!(result, "hello from affinity");
    }

    #[test]
    fn test_graceful_channel_submit_reentrant_after_shutdown() {
        let name = format!("test_reentrant_shutdown_{}", std::process::id());
        let channel = GracefulIpcChannel::<Vec<u8>>::create(&name).unwrap();
        channel.shutdown();

        let result = channel.submit_reentrant(|| ());
        assert!(matches!(result, Err(IpcError::Closed)));
    }
}
