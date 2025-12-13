//! Graceful shutdown mechanism for IPC channels
//!
//! This module provides the `GracefulChannel` trait for implementing graceful shutdown
//! in IPC channels, preventing errors like `EventLoopClosed` when background threads
//! continue sending messages after the main event loop has closed.
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
// GracefulIpcChannel - IPC channel with graceful shutdown
// ============================================================================

use crate::channel::IpcChannel;
use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;

/// IPC channel with graceful shutdown support
pub struct GracefulIpcChannel<T = Vec<u8>> {
    inner: IpcChannel<T>,
    state: Arc<ShutdownState>,
    _marker: PhantomData<T>,
}

impl<T> GracefulIpcChannel<T> {
    /// Create a new graceful IPC channel wrapper
    pub fn new(channel: IpcChannel<T>) -> Self {
        Self {
            inner: channel,
            state: Arc::new(ShutdownState::new()),
            _marker: PhantomData,
        }
    }

    /// Create a new graceful IPC channel with a shared shutdown state
    pub fn with_state(channel: IpcChannel<T>, state: Arc<ShutdownState>) -> Self {
        Self {
            inner: channel,
            state,
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
}
