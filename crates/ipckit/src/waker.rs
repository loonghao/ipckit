//! # Event Loop Waker
//!
//! This module provides traits and implementations for waking up event loops
//! when IPC messages arrive. This is essential for integrating ipckit with
//! GUI frameworks (Qt, GTK, winit/tao) and async runtimes (tokio).
//!
//! ## Example
//!
//! ```rust,ignore
//! use ipckit::{EventLoopWaker, ThreadWaker, WakeableChannel};
//!
//! // Create a waker for the current thread
//! let waker = ThreadWaker::current();
//!
//! // Set the waker on a channel
//! channel.set_waker(Box::new(waker));
//!
//! // Now when messages arrive, the thread will be woken
//! ```

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::Thread;

#[cfg(feature = "async")]
use tokio::sync::Notify;

/// Trait for waking up an event loop when IPC messages arrive.
///
/// Implementations of this trait can be used to integrate ipckit with
/// various event loop systems like GUI frameworks or async runtimes.
pub trait EventLoopWaker: Send + Sync {
    /// Wake up the event loop.
    ///
    /// This method should be called when new IPC messages are available
    /// and the event loop needs to process them.
    fn wake(&self);

    /// Check if the waker is still valid.
    ///
    /// Returns `false` if the associated event loop has been closed
    /// or the waker can no longer wake it up.
    fn is_valid(&self) -> bool;

    /// Clone the waker into a boxed trait object.
    fn clone_box(&self) -> Box<dyn EventLoopWaker>;
}

impl Clone for Box<dyn EventLoopWaker> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// A waker that wakes a specific thread using `std::thread::Thread::unpark()`.
///
/// This is useful for simple blocking scenarios where a thread is waiting
/// for IPC messages using `std::thread::park()`.
#[derive(Debug, Clone)]
pub struct ThreadWaker {
    thread: Thread,
    valid: Arc<AtomicBool>,
}

impl ThreadWaker {
    /// Create a waker for the current thread.
    pub fn current() -> Self {
        Self {
            thread: std::thread::current(),
            valid: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Create a waker for a specific thread.
    pub fn new(thread: Thread) -> Self {
        Self {
            thread,
            valid: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Invalidate this waker.
    ///
    /// After calling this, `is_valid()` will return `false`.
    pub fn invalidate(&self) {
        self.valid.store(false, Ordering::SeqCst);
    }
}

impl EventLoopWaker for ThreadWaker {
    fn wake(&self) {
        if self.is_valid() {
            self.thread.unpark();
        }
    }

    fn is_valid(&self) -> bool {
        self.valid.load(Ordering::SeqCst)
    }

    fn clone_box(&self) -> Box<dyn EventLoopWaker> {
        Box::new(self.clone())
    }
}

/// A waker that uses a callback function.
///
/// This is useful for integrating with custom event loop systems.
pub struct CallbackWaker<F>
where
    F: Fn() + Send + Sync + Clone + 'static,
{
    callback: F,
    valid: Arc<AtomicBool>,
}

impl<F> CallbackWaker<F>
where
    F: Fn() + Send + Sync + Clone + 'static,
{
    /// Create a new callback waker.
    pub fn new(callback: F) -> Self {
        Self {
            callback,
            valid: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Invalidate this waker.
    pub fn invalidate(&self) {
        self.valid.store(false, Ordering::SeqCst);
    }
}

impl<F> Clone for CallbackWaker<F>
where
    F: Fn() + Send + Sync + Clone + 'static,
{
    fn clone(&self) -> Self {
        Self {
            callback: self.callback.clone(),
            valid: Arc::clone(&self.valid),
        }
    }
}

impl<F> EventLoopWaker for CallbackWaker<F>
where
    F: Fn() + Send + Sync + Clone + 'static,
{
    fn wake(&self) {
        if self.is_valid() {
            (self.callback)();
        }
    }

    fn is_valid(&self) -> bool {
        self.valid.load(Ordering::SeqCst)
    }

    fn clone_box(&self) -> Box<dyn EventLoopWaker> {
        Box::new(self.clone())
    }
}

/// A waker for tokio async runtime.
///
/// Uses `tokio::sync::Notify` to wake up async tasks waiting for IPC messages.
#[cfg(feature = "async")]
#[derive(Debug, Clone)]
pub struct TokioWaker {
    notify: Arc<Notify>,
    valid: Arc<AtomicBool>,
}

#[cfg(feature = "async")]
impl TokioWaker {
    /// Create a new tokio waker.
    pub fn new() -> Self {
        Self {
            notify: Arc::new(Notify::new()),
            valid: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Get the notify handle for waiting.
    ///
    /// Use this to await notifications in async code:
    /// ```rust,ignore
    /// waker.notified().await;
    /// ```
    pub fn notified(&self) -> tokio::sync::futures::Notified<'_> {
        self.notify.notified()
    }

    /// Invalidate this waker.
    pub fn invalidate(&self) {
        self.valid.store(false, Ordering::SeqCst);
    }
}

#[cfg(feature = "async")]
impl Default for TokioWaker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "async")]
impl EventLoopWaker for TokioWaker {
    fn wake(&self) {
        if self.is_valid() {
            self.notify.notify_one();
        }
    }

    fn is_valid(&self) -> bool {
        self.valid.load(Ordering::SeqCst)
    }

    fn clone_box(&self) -> Box<dyn EventLoopWaker> {
        Box::new(self.clone())
    }
}

/// A waker that broadcasts to multiple wakers.
///
/// Useful when multiple event loops need to be notified of the same event.
#[derive(Clone, Default)]
pub struct BroadcastWaker {
    wakers: Vec<Box<dyn EventLoopWaker>>,
}

impl BroadcastWaker {
    /// Create a new broadcast waker.
    pub fn new() -> Self {
        Self { wakers: Vec::new() }
    }

    /// Add a waker to the broadcast list.
    pub fn add(&mut self, waker: Box<dyn EventLoopWaker>) {
        self.wakers.push(waker);
    }

    /// Remove invalid wakers from the list.
    pub fn cleanup(&mut self) {
        self.wakers.retain(|w| w.is_valid());
    }

    /// Get the number of wakers.
    pub fn len(&self) -> usize {
        self.wakers.len()
    }

    /// Check if there are no wakers.
    pub fn is_empty(&self) -> bool {
        self.wakers.is_empty()
    }
}

impl EventLoopWaker for BroadcastWaker {
    fn wake(&self) {
        for waker in &self.wakers {
            if waker.is_valid() {
                waker.wake();
            }
        }
    }

    fn is_valid(&self) -> bool {
        self.wakers.iter().any(|w| w.is_valid())
    }

    fn clone_box(&self) -> Box<dyn EventLoopWaker> {
        Box::new(self.clone())
    }
}

/// A channel that can wake an event loop when messages arrive.
pub trait WakeableChannel {
    /// Set the event loop waker.
    ///
    /// When messages arrive on this channel, the waker will be called
    /// to notify the event loop.
    fn set_waker(&mut self, waker: Box<dyn EventLoopWaker>);

    /// Remove the waker.
    fn clear_waker(&mut self);

    /// Get a reference to the current waker, if any.
    fn waker(&self) -> Option<&dyn EventLoopWaker>;
}

/// A wrapper that adds waker support to any channel.
pub struct WakeableWrapper<C> {
    inner: C,
    waker: Option<Box<dyn EventLoopWaker>>,
}

impl<C> WakeableWrapper<C> {
    /// Create a new wakeable wrapper around a channel.
    pub fn new(channel: C) -> Self {
        Self {
            inner: channel,
            waker: None,
        }
    }

    /// Get a reference to the inner channel.
    pub fn inner(&self) -> &C {
        &self.inner
    }

    /// Get a mutable reference to the inner channel.
    pub fn inner_mut(&mut self) -> &mut C {
        &mut self.inner
    }

    /// Consume the wrapper and return the inner channel.
    pub fn into_inner(self) -> C {
        self.inner
    }

    /// Wake the event loop if a waker is set.
    pub fn wake(&self) {
        if let Some(ref waker) = self.waker {
            if waker.is_valid() {
                waker.wake();
            }
        }
    }
}

impl<C> WakeableChannel for WakeableWrapper<C> {
    fn set_waker(&mut self, waker: Box<dyn EventLoopWaker>) {
        self.waker = Some(waker);
    }

    fn clear_waker(&mut self) {
        self.waker = None;
    }

    fn waker(&self) -> Option<&dyn EventLoopWaker> {
        self.waker.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::time::Duration;

    #[test]
    fn test_thread_waker() {
        let waker = ThreadWaker::current();
        assert!(waker.is_valid());

        waker.wake();
        // Should not panic

        waker.invalidate();
        assert!(!waker.is_valid());
    }

    #[test]
    fn test_callback_waker() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        let waker = CallbackWaker::new(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        assert!(waker.is_valid());
        waker.wake();
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        waker.wake();
        assert_eq!(counter.load(Ordering::SeqCst), 2);

        waker.invalidate();
        waker.wake();
        assert_eq!(counter.load(Ordering::SeqCst), 2); // Should not increment
    }

    #[test]
    fn test_broadcast_waker() {
        let counter1 = Arc::new(AtomicUsize::new(0));
        let counter2 = Arc::new(AtomicUsize::new(0));

        let c1 = Arc::clone(&counter1);
        let c2 = Arc::clone(&counter2);

        let mut broadcast = BroadcastWaker::new();
        broadcast.add(Box::new(CallbackWaker::new(move || {
            c1.fetch_add(1, Ordering::SeqCst);
        })));
        broadcast.add(Box::new(CallbackWaker::new(move || {
            c2.fetch_add(1, Ordering::SeqCst);
        })));

        assert_eq!(broadcast.len(), 2);
        assert!(broadcast.is_valid());

        broadcast.wake();
        assert_eq!(counter1.load(Ordering::SeqCst), 1);
        assert_eq!(counter2.load(Ordering::SeqCst), 1);
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_tokio_waker() {
        let waker = TokioWaker::new();
        assert!(waker.is_valid());

        let waker_clone = waker.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            waker_clone.wake();
        });

        tokio::time::timeout(Duration::from_millis(100), waker.notified())
            .await
            .expect("Should be notified");
    }

    #[test]
    fn test_wakeable_wrapper() {
        struct DummyChannel;

        let mut wrapper = WakeableWrapper::new(DummyChannel);
        assert!(wrapper.waker().is_none());

        let counter = Arc::new(AtomicUsize::new(0));
        let c = Arc::clone(&counter);
        wrapper.set_waker(Box::new(CallbackWaker::new(move || {
            c.fetch_add(1, Ordering::SeqCst);
        })));

        assert!(wrapper.waker().is_some());
        wrapper.wake();
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        wrapper.clear_waker();
        assert!(wrapper.waker().is_none());
    }
}
