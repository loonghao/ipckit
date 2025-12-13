//! Thread Channel for intra-process thread communication
//!
//! This module provides a high-performance channel for communication between threads
//! within the same process, using crossbeam-channel as the underlying implementation.
//!
//! # Example
//!
//! ```rust
//! use ipckit::ThreadChannel;
//! use std::thread;
//!
//! // Create an unbounded channel
//! let (tx, rx) = ThreadChannel::<String>::unbounded();
//!
//! thread::spawn(move || {
//!     tx.send("Hello from thread!".to_string()).unwrap();
//! });
//!
//! let msg = rx.recv().unwrap();
//! assert_eq!(msg, "Hello from thread!");
//! ```

use crate::error::{IpcError, Result};
use crate::graceful::{GracefulChannel, ShutdownState};
use crossbeam_channel::{self, Receiver, RecvTimeoutError, Sender, TryRecvError, TrySendError};
use std::sync::Arc;
use std::time::Duration;

/// A thread-safe channel sender for intra-process communication.
///
/// This is the sending half of a [`ThreadChannel`]. It can be cloned to create
/// multiple producers that send to the same channel.
#[derive(Debug)]
pub struct ThreadSender<T> {
    inner: Sender<T>,
    shutdown: Arc<ShutdownState>,
}

/// A thread-safe channel receiver for intra-process communication.
///
/// This is the receiving half of a [`ThreadChannel`]. It can be cloned to create
/// multiple consumers that receive from the same channel.
#[derive(Debug)]
pub struct ThreadReceiver<T> {
    inner: Receiver<T>,
    shutdown: Arc<ShutdownState>,
}

impl<T> Clone for ThreadSender<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            shutdown: Arc::clone(&self.shutdown),
        }
    }
}

impl<T> Clone for ThreadReceiver<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            shutdown: Arc::clone(&self.shutdown),
        }
    }
}

impl<T> ThreadSender<T> {
    /// Send a message through the channel.
    ///
    /// This method blocks if the channel is bounded and full.
    ///
    /// # Errors
    ///
    /// Returns `IpcError::Closed` if the channel has been shutdown or all receivers have been dropped.
    pub fn send(&self, msg: T) -> Result<()> {
        if self.shutdown.is_shutdown() {
            return Err(IpcError::Closed);
        }

        self.inner.send(msg).map_err(|_| IpcError::Closed)
    }

    /// Try to send a message without blocking.
    ///
    /// # Errors
    ///
    /// - `IpcError::Closed` if the channel has been shutdown or all receivers have been dropped.
    /// - `IpcError::WouldBlock` if the channel is full (bounded channels only).
    pub fn try_send(&self, msg: T) -> Result<()> {
        if self.shutdown.is_shutdown() {
            return Err(IpcError::Closed);
        }

        self.inner.try_send(msg).map_err(|e| match e {
            TrySendError::Full(_) => IpcError::WouldBlock,
            TrySendError::Disconnected(_) => IpcError::Closed,
        })
    }

    /// Send a message with a timeout.
    ///
    /// # Errors
    ///
    /// - `IpcError::Closed` if the channel has been shutdown or all receivers have been dropped.
    /// - `IpcError::Timeout` if the timeout expires before the message can be sent.
    pub fn send_timeout(&self, msg: T, timeout: Duration) -> Result<()> {
        if self.shutdown.is_shutdown() {
            return Err(IpcError::Closed);
        }

        self.inner.send_timeout(msg, timeout).map_err(|e| {
            if e.is_timeout() {
                IpcError::Timeout
            } else {
                IpcError::Closed
            }
        })
    }

    /// Check if the channel is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Check if the channel is full (always false for unbounded channels).
    pub fn is_full(&self) -> bool {
        self.inner.is_full()
    }

    /// Get the number of messages in the channel.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Get the capacity of the channel (None for unbounded channels).
    pub fn capacity(&self) -> Option<usize> {
        self.inner.capacity()
    }

    /// Check if the channel has been shutdown.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.is_shutdown()
    }

    /// Shutdown the channel.
    pub fn shutdown(&self) {
        self.shutdown.shutdown();
    }
}

impl<T> ThreadReceiver<T> {
    /// Receive a message from the channel.
    ///
    /// This method blocks until a message is available.
    ///
    /// # Errors
    ///
    /// Returns `IpcError::Closed` if the channel has been shutdown or all senders have been dropped.
    pub fn recv(&self) -> Result<T> {
        if self.shutdown.is_shutdown() {
            // Try to drain remaining messages first
            return self.inner.try_recv().map_err(|_| IpcError::Closed);
        }

        self.inner.recv().map_err(|_| IpcError::Closed)
    }

    /// Try to receive a message without blocking.
    ///
    /// # Errors
    ///
    /// - `IpcError::Closed` if the channel has been shutdown or all senders have been dropped.
    /// - `IpcError::WouldBlock` if no message is available.
    pub fn try_recv(&self) -> Result<T> {
        self.inner.try_recv().map_err(|e| match e {
            TryRecvError::Empty => IpcError::WouldBlock,
            TryRecvError::Disconnected => IpcError::Closed,
        })
    }

    /// Receive a message with a timeout.
    ///
    /// # Errors
    ///
    /// - `IpcError::Closed` if the channel has been shutdown or all senders have been dropped.
    /// - `IpcError::Timeout` if the timeout expires before a message is available.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<T> {
        if self.shutdown.is_shutdown() {
            return self.try_recv();
        }

        self.inner.recv_timeout(timeout).map_err(|e| match e {
            RecvTimeoutError::Timeout => IpcError::Timeout,
            RecvTimeoutError::Disconnected => IpcError::Closed,
        })
    }

    /// Check if the channel is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get the number of messages in the channel.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Get the capacity of the channel (None for unbounded channels).
    pub fn capacity(&self) -> Option<usize> {
        self.inner.capacity()
    }

    /// Check if the channel has been shutdown.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.is_shutdown()
    }

    /// Shutdown the channel.
    pub fn shutdown(&self) {
        self.shutdown.shutdown();
    }

    /// Create an iterator over received messages.
    ///
    /// The iterator will block waiting for messages and will stop when the channel is closed.
    pub fn iter(&self) -> impl Iterator<Item = T> + '_ {
        std::iter::from_fn(move || self.recv().ok())
    }

    /// Create a non-blocking iterator over available messages.
    ///
    /// The iterator will return `None` when no more messages are immediately available.
    pub fn try_iter(&self) -> impl Iterator<Item = T> + '_ {
        std::iter::from_fn(move || self.try_recv().ok())
    }
}

/// A bidirectional thread channel that combines both sender and receiver.
///
/// This is useful when you need both send and receive capabilities in one place.
#[derive(Debug)]
pub struct ThreadChannel<T> {
    sender: ThreadSender<T>,
    receiver: ThreadReceiver<T>,
}

impl<T> ThreadChannel<T> {
    /// Create a new unbounded thread channel.
    ///
    /// An unbounded channel has no capacity limit and will never block on send.
    ///
    /// # Returns
    ///
    /// A tuple of (sender, receiver) for the channel.
    pub fn unbounded() -> (ThreadSender<T>, ThreadReceiver<T>) {
        let (tx, rx) = crossbeam_channel::unbounded();
        let shutdown = Arc::new(ShutdownState::new());

        let sender = ThreadSender {
            inner: tx,
            shutdown: Arc::clone(&shutdown),
        };

        let receiver = ThreadReceiver {
            inner: rx,
            shutdown,
        };

        (sender, receiver)
    }

    /// Create a new bounded thread channel with the specified capacity.
    ///
    /// A bounded channel will block on send when the channel is full.
    ///
    /// # Arguments
    ///
    /// * `capacity` - The maximum number of messages the channel can hold.
    ///
    /// # Returns
    ///
    /// A tuple of (sender, receiver) for the channel.
    pub fn bounded(capacity: usize) -> (ThreadSender<T>, ThreadReceiver<T>) {
        let (tx, rx) = crossbeam_channel::bounded(capacity);
        let shutdown = Arc::new(ShutdownState::new());

        let sender = ThreadSender {
            inner: tx,
            shutdown: Arc::clone(&shutdown),
        };

        let receiver = ThreadReceiver {
            inner: rx,
            shutdown,
        };

        (sender, receiver)
    }

    /// Create a new bidirectional thread channel (unbounded).
    pub fn new_unbounded() -> Self {
        let (sender, receiver) = Self::unbounded();
        Self { sender, receiver }
    }

    /// Create a new bidirectional thread channel (bounded).
    pub fn new_bounded(capacity: usize) -> Self {
        let (sender, receiver) = Self::bounded(capacity);
        Self { sender, receiver }
    }

    /// Get a reference to the sender.
    pub fn sender(&self) -> &ThreadSender<T> {
        &self.sender
    }

    /// Get a reference to the receiver.
    pub fn receiver(&self) -> &ThreadReceiver<T> {
        &self.receiver
    }

    /// Clone the sender.
    pub fn clone_sender(&self) -> ThreadSender<T> {
        self.sender.clone()
    }

    /// Clone the receiver.
    pub fn clone_receiver(&self) -> ThreadReceiver<T> {
        self.receiver.clone()
    }

    /// Split the channel into sender and receiver.
    pub fn split(self) -> (ThreadSender<T>, ThreadReceiver<T>) {
        (self.sender, self.receiver)
    }
}

impl<T> GracefulChannel for ThreadChannel<T> {
    fn shutdown(&self) {
        self.sender.shutdown();
    }

    fn is_shutdown(&self) -> bool {
        self.sender.is_shutdown()
    }

    fn drain(&self) -> Result<()> {
        // For thread channels, drain means receiving all pending messages
        while self.receiver.try_recv().is_ok() {}
        Ok(())
    }

    fn shutdown_timeout(&self, timeout: Duration) -> Result<()> {
        self.shutdown();
        let start = std::time::Instant::now();

        while !self.receiver.is_empty() {
            if start.elapsed() >= timeout {
                return Err(IpcError::Timeout);
            }
            let _ = self.receiver.try_recv();
            std::thread::sleep(Duration::from_millis(1));
        }

        Ok(())
    }
}

impl<T> GracefulChannel for ThreadSender<T> {
    fn shutdown(&self) {
        self.shutdown.shutdown();
    }

    fn is_shutdown(&self) -> bool {
        self.shutdown.is_shutdown()
    }

    fn drain(&self) -> Result<()> {
        self.shutdown.wait_for_drain(None)
    }

    fn shutdown_timeout(&self, timeout: Duration) -> Result<()> {
        self.shutdown();
        self.shutdown.wait_for_drain(Some(timeout))
    }
}

impl<T> GracefulChannel for ThreadReceiver<T> {
    fn shutdown(&self) {
        self.shutdown.shutdown();
    }

    fn is_shutdown(&self) -> bool {
        self.shutdown.is_shutdown()
    }

    fn drain(&self) -> Result<()> {
        while self.try_recv().is_ok() {}
        Ok(())
    }

    fn shutdown_timeout(&self, timeout: Duration) -> Result<()> {
        self.shutdown();
        let start = std::time::Instant::now();

        while !self.is_empty() {
            if start.elapsed() >= timeout {
                return Err(IpcError::Timeout);
            }
            let _ = self.try_recv();
            std::thread::sleep(Duration::from_millis(1));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_unbounded_channel() {
        let (tx, rx) = ThreadChannel::<i32>::unbounded();

        tx.send(42).unwrap();
        tx.send(43).unwrap();

        assert_eq!(rx.recv().unwrap(), 42);
        assert_eq!(rx.recv().unwrap(), 43);
    }

    #[test]
    fn test_bounded_channel() {
        let (tx, rx) = ThreadChannel::<i32>::bounded(2);

        tx.send(1).unwrap();
        tx.send(2).unwrap();

        // Channel is full, try_send should fail
        assert!(matches!(tx.try_send(3), Err(IpcError::WouldBlock)));

        assert_eq!(rx.recv().unwrap(), 1);

        // Now we can send again
        tx.send(3).unwrap();

        assert_eq!(rx.recv().unwrap(), 2);
        assert_eq!(rx.recv().unwrap(), 3);
    }

    #[test]
    fn test_multi_producer() {
        let (tx, rx) = ThreadChannel::<i32>::unbounded();
        let tx2 = tx.clone();

        let h1 = thread::spawn(move || {
            for i in 0..5 {
                tx.send(i).unwrap();
            }
        });

        let h2 = thread::spawn(move || {
            for i in 5..10 {
                tx2.send(i).unwrap();
            }
        });

        h1.join().unwrap();
        h2.join().unwrap();

        let mut received: Vec<i32> = rx.try_iter().collect();
        received.sort();

        assert_eq!(received, (0..10).collect::<Vec<_>>());
    }

    #[test]
    fn test_multi_consumer() {
        let (tx, rx) = ThreadChannel::<i32>::unbounded();
        let rx2 = rx.clone();

        for i in 0..10 {
            tx.send(i).unwrap();
        }
        drop(tx);

        let h1 = thread::spawn(move || {
            let mut received = Vec::new();
            while let Ok(v) = rx.recv() {
                received.push(v);
            }
            received
        });

        let h2 = thread::spawn(move || {
            let mut received = Vec::new();
            while let Ok(v) = rx2.recv() {
                received.push(v);
            }
            received
        });

        let r1 = h1.join().unwrap();
        let r2 = h2.join().unwrap();

        let mut all: Vec<i32> = r1.into_iter().chain(r2).collect();
        all.sort();

        assert_eq!(all, (0..10).collect::<Vec<_>>());
    }

    #[test]
    fn test_shutdown() {
        let (tx, rx) = ThreadChannel::<i32>::unbounded();

        tx.send(1).unwrap();
        tx.shutdown();

        // Should fail after shutdown
        assert!(matches!(tx.send(2), Err(IpcError::Closed)));

        // Can still receive pending messages
        assert_eq!(rx.recv().unwrap(), 1);
    }

    #[test]
    fn test_recv_timeout() {
        let (_tx, rx) = ThreadChannel::<i32>::unbounded();

        let result = rx.recv_timeout(Duration::from_millis(50));
        assert!(matches!(result, Err(IpcError::Timeout)));
    }

    #[test]
    fn test_send_timeout() {
        let (tx, _rx) = ThreadChannel::<i32>::bounded(1);

        tx.send(1).unwrap();

        let result = tx.send_timeout(2, Duration::from_millis(50));
        assert!(matches!(result, Err(IpcError::Timeout)));
    }

    #[test]
    fn test_try_recv() {
        let (tx, rx) = ThreadChannel::<i32>::unbounded();

        assert!(matches!(rx.try_recv(), Err(IpcError::WouldBlock)));

        tx.send(42).unwrap();

        assert_eq!(rx.try_recv().unwrap(), 42);
        assert!(matches!(rx.try_recv(), Err(IpcError::WouldBlock)));
    }

    #[test]
    fn test_channel_capacity() {
        let (tx, rx) = ThreadChannel::<i32>::bounded(5);

        assert_eq!(tx.capacity(), Some(5));
        assert_eq!(rx.capacity(), Some(5));
        assert!(tx.is_empty());
        assert!(!tx.is_full());

        for i in 0..5 {
            tx.send(i).unwrap();
        }

        assert!(tx.is_full());
        assert!(!tx.is_empty());
        assert_eq!(tx.len(), 5);
    }

    #[test]
    fn test_unbounded_capacity() {
        let (tx, rx) = ThreadChannel::<i32>::unbounded();

        assert_eq!(tx.capacity(), None);
        assert_eq!(rx.capacity(), None);
        assert!(!tx.is_full()); // Unbounded is never full
    }

    #[test]
    fn test_graceful_channel_trait() {
        let channel = ThreadChannel::<i32>::new_unbounded();

        assert!(!channel.is_shutdown());

        channel.sender().send(1).unwrap();
        channel.sender().send(2).unwrap();

        channel.shutdown();

        assert!(channel.is_shutdown());

        // Drain remaining messages
        channel.drain().unwrap();

        assert!(channel.receiver().is_empty());
    }

    #[test]
    fn test_iter() {
        let (tx, rx) = ThreadChannel::<i32>::unbounded();

        tx.send(1).unwrap();
        tx.send(2).unwrap();
        tx.send(3).unwrap();
        drop(tx);

        let collected: Vec<i32> = rx.iter().collect();
        assert_eq!(collected, vec![1, 2, 3]);
    }

    #[test]
    fn test_try_iter() {
        let (tx, rx) = ThreadChannel::<i32>::unbounded();

        tx.send(1).unwrap();
        tx.send(2).unwrap();

        let collected: Vec<i32> = rx.try_iter().collect();
        assert_eq!(collected, vec![1, 2]);

        // try_iter doesn't block, so we can continue
        tx.send(3).unwrap();
        assert_eq!(rx.recv().unwrap(), 3);
    }
}
