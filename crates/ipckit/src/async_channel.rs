//! # Async Channel Support
//!
//! This module provides async/await support for IPC channels using tokio.
//! It enables non-blocking, async-first IPC communication.
//!
//! ## Features
//!
//! - Async channel traits
//! - Tokio integration
//! - Stream-based message receiving
//! - Async timeout support
//!
//! ## Example
//!
//! ```rust,ignore
//! use ipckit::async_channel::{AsyncIpcChannel, AsyncIpcSender, AsyncIpcReceiver};
//!
//! async fn example() -> ipckit::Result<()> {
//!     let (tx, rx) = AsyncThreadChannel::<String>::unbounded();
//!
//!     tx.send("Hello".to_string()).await?;
//!     let msg = rx.recv().await?;
//!
//!     Ok(())
//! }
//! ```

use crate::error::{IpcError, Result};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

/// Async IPC sender trait.
///
/// Provides async send capabilities for IPC channels.
pub trait AsyncIpcSender: Send + Sync {
    /// The message type being sent.
    type Message: Send;

    /// Send a message asynchronously.
    fn send(&self, msg: Self::Message) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;

    /// Try to send a message without blocking.
    fn try_send(&self, msg: Self::Message) -> Result<()>;
}

/// Async IPC receiver trait.
///
/// Provides async receive capabilities for IPC channels.
pub trait AsyncIpcReceiver: Send + Sync {
    /// The message type being received.
    type Message: Send;

    /// Receive a message asynchronously.
    fn recv(&self) -> Pin<Box<dyn Future<Output = Result<Self::Message>> + Send + '_>>;

    /// Receive a message with timeout.
    fn recv_timeout(
        &self,
        timeout: Duration,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Message>> + Send + '_>>;

    /// Try to receive a message without blocking.
    fn try_recv(&self) -> Result<Option<Self::Message>>;
}

/// Async bidirectional IPC channel trait.
pub trait AsyncIpcChannel: AsyncIpcSender + AsyncIpcReceiver {}

// Blanket implementation
impl<T> AsyncIpcChannel for T where T: AsyncIpcSender + AsyncIpcReceiver {}

/// Async thread channel using tokio's mpsc.
#[cfg(feature = "async")]
pub mod tokio_channel {
    use super::*;
    use crate::graceful::ShutdownState;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    /// Async sender for thread-local communication.
    pub struct AsyncThreadSender<T> {
        inner: mpsc::Sender<T>,
        shutdown: Arc<ShutdownState>,
    }

    /// Async receiver for thread-local communication.
    pub struct AsyncThreadReceiver<T> {
        inner: mpsc::Receiver<T>,
        shutdown: Arc<ShutdownState>,
    }

    impl<T> Clone for AsyncThreadSender<T> {
        fn clone(&self) -> Self {
            Self {
                inner: self.inner.clone(),
                shutdown: Arc::clone(&self.shutdown),
            }
        }
    }

    impl<T: Send + 'static> AsyncThreadSender<T> {
        /// Send a message asynchronously.
        pub async fn send(&self, msg: T) -> Result<()> {
            if self.shutdown.is_shutdown() {
                return Err(IpcError::Closed);
            }
            self.inner.send(msg).await.map_err(|_| IpcError::Closed)
        }

        /// Try to send without waiting.
        pub fn try_send(&self, msg: T) -> Result<()> {
            if self.shutdown.is_shutdown() {
                return Err(IpcError::Closed);
            }
            self.inner.try_send(msg).map_err(|e| match e {
                mpsc::error::TrySendError::Full(_) => IpcError::WouldBlock,
                mpsc::error::TrySendError::Closed(_) => IpcError::Closed,
            })
        }

        /// Check if the channel is closed.
        pub fn is_closed(&self) -> bool {
            self.inner.is_closed()
        }

        /// Shutdown the channel.
        pub fn shutdown(&self) {
            self.shutdown.shutdown();
        }
    }

    impl<T: Send + 'static> AsyncThreadReceiver<T> {
        /// Receive a message asynchronously.
        pub async fn recv(&mut self) -> Result<T> {
            if self.shutdown.is_shutdown() {
                return Err(IpcError::Closed);
            }
            self.inner.recv().await.ok_or(IpcError::Closed)
        }

        /// Receive with timeout.
        pub async fn recv_timeout(&mut self, timeout: Duration) -> Result<T> {
            if self.shutdown.is_shutdown() {
                return Err(IpcError::Closed);
            }
            tokio::time::timeout(timeout, self.inner.recv())
                .await
                .map_err(|_| IpcError::Timeout)?
                .ok_or(IpcError::Closed)
        }

        /// Try to receive without waiting.
        pub fn try_recv(&mut self) -> Result<Option<T>> {
            if self.shutdown.is_shutdown() {
                return Err(IpcError::Closed);
            }
            match self.inner.try_recv() {
                Ok(msg) => Ok(Some(msg)),
                Err(mpsc::error::TryRecvError::Empty) => Ok(None),
                Err(mpsc::error::TryRecvError::Disconnected) => Err(IpcError::Closed),
            }
        }

        /// Check if the channel is closed.
        pub fn is_closed(&self) -> bool {
            self.shutdown.is_shutdown()
        }

        /// Shutdown the channel.
        pub fn shutdown(&self) {
            self.shutdown.shutdown();
        }
    }

    /// Async thread channel factory.
    pub struct AsyncThreadChannel<T>(std::marker::PhantomData<T>);

    impl<T: Send + 'static> AsyncThreadChannel<T> {
        /// Create an unbounded async channel.
        ///
        /// Note: Uses a large but safe buffer size (1 million) instead of usize::MAX
        /// because tokio's semaphore has a maximum permit limit.
        pub fn unbounded() -> (AsyncThreadSender<T>, AsyncThreadReceiver<T>) {
            // tokio's mpsc uses a semaphore internally which has MAX_PERMITS limit
            // Using a large but safe value instead of usize::MAX
            const LARGE_BUFFER: usize = 1_000_000;
            let (tx, rx) = mpsc::channel(LARGE_BUFFER);
            let shutdown = Arc::new(ShutdownState::new());

            (
                AsyncThreadSender {
                    inner: tx,
                    shutdown: Arc::clone(&shutdown),
                },
                AsyncThreadReceiver {
                    inner: rx,
                    shutdown,
                },
            )
        }

        /// Create a bounded async channel.
        pub fn bounded(capacity: usize) -> (AsyncThreadSender<T>, AsyncThreadReceiver<T>) {
            let (tx, rx) = mpsc::channel(capacity);
            let shutdown = Arc::new(ShutdownState::new());

            (
                AsyncThreadSender {
                    inner: tx,
                    shutdown: Arc::clone(&shutdown),
                },
                AsyncThreadReceiver {
                    inner: rx,
                    shutdown,
                },
            )
        }
    }

    /// Spawn a handler on the tokio runtime.
    pub fn spawn_handler<T, F, Fut>(
        mut receiver: AsyncThreadReceiver<T>,
        handler: F,
    ) -> tokio::task::JoinHandle<()>
    where
        T: Send + 'static,
        F: Fn(T) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send,
    {
        tokio::spawn(async move {
            while let Ok(msg) = receiver.recv().await {
                handler(msg).await;
            }
        })
    }
}

/// Async oneshot channel for single-value communication.
#[cfg(feature = "async")]
pub mod oneshot {
    use super::*;
    use tokio::sync::oneshot as tokio_oneshot;

    /// Sender for a one-shot channel.
    pub struct OneshotSender<T> {
        inner: tokio_oneshot::Sender<T>,
    }

    /// Receiver for a one-shot channel.
    pub struct OneshotReceiver<T> {
        inner: tokio_oneshot::Receiver<T>,
    }

    impl<T> OneshotSender<T> {
        /// Send a value, consuming the sender.
        pub fn send(self, value: T) -> Result<()> {
            self.inner.send(value).map_err(|_| IpcError::Closed)
        }

        /// Check if the receiver is still waiting.
        pub fn is_closed(&self) -> bool {
            self.inner.is_closed()
        }
    }

    impl<T> OneshotReceiver<T> {
        /// Receive the value asynchronously.
        pub async fn recv(self) -> Result<T> {
            self.inner.await.map_err(|_| IpcError::Closed)
        }

        /// Try to receive without waiting.
        pub fn try_recv(&mut self) -> Result<Option<T>> {
            match self.inner.try_recv() {
                Ok(v) => Ok(Some(v)),
                Err(tokio_oneshot::error::TryRecvError::Empty) => Ok(None),
                Err(tokio_oneshot::error::TryRecvError::Closed) => Err(IpcError::Closed),
            }
        }
    }

    /// Create a new oneshot channel.
    pub fn channel<T>() -> (OneshotSender<T>, OneshotReceiver<T>) {
        let (tx, rx) = tokio_oneshot::channel();
        (OneshotSender { inner: tx }, OneshotReceiver { inner: rx })
    }
}

/// Async broadcast channel for pub/sub patterns.
#[cfg(feature = "async")]
pub mod broadcast {
    use super::*;
    use tokio::sync::broadcast as tokio_broadcast;

    /// Sender for a broadcast channel.
    #[derive(Clone)]
    pub struct BroadcastSender<T: Clone> {
        inner: tokio_broadcast::Sender<T>,
    }

    /// Receiver for a broadcast channel.
    pub struct BroadcastReceiver<T: Clone> {
        inner: tokio_broadcast::Receiver<T>,
    }

    impl<T: Clone + Send + 'static> BroadcastSender<T> {
        /// Send a value to all receivers.
        pub fn send(&self, value: T) -> Result<usize> {
            self.inner.send(value).map_err(|_| IpcError::Closed)
        }

        /// Get the number of active receivers.
        pub fn receiver_count(&self) -> usize {
            self.inner.receiver_count()
        }

        /// Create a new receiver for this channel.
        pub fn subscribe(&self) -> BroadcastReceiver<T> {
            BroadcastReceiver {
                inner: self.inner.subscribe(),
            }
        }
    }

    impl<T: Clone + Send + 'static> BroadcastReceiver<T> {
        /// Receive a value asynchronously.
        pub async fn recv(&mut self) -> Result<T> {
            loop {
                match self.inner.recv().await {
                    Ok(v) => return Ok(v),
                    Err(tokio_broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio_broadcast::error::RecvError::Closed) => return Err(IpcError::Closed),
                }
            }
        }
    }

    /// Create a new broadcast channel.
    pub fn channel<T: Clone>(capacity: usize) -> (BroadcastSender<T>, BroadcastReceiver<T>) {
        let (tx, rx) = tokio_broadcast::channel(capacity);
        (
            BroadcastSender { inner: tx },
            BroadcastReceiver { inner: rx },
        )
    }
}

#[cfg(all(test, feature = "async"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_async_thread_channel() {
        use tokio_channel::AsyncThreadChannel;

        let (tx, mut rx) = AsyncThreadChannel::<String>::unbounded();

        tx.send("Hello".to_string()).await.unwrap();
        let msg = rx.recv().await.unwrap();
        assert_eq!(msg, "Hello");
    }

    #[tokio::test]
    async fn test_async_thread_channel_timeout() {
        use tokio_channel::AsyncThreadChannel;

        let (_tx, mut rx) = AsyncThreadChannel::<String>::bounded(1);

        let result = rx.recv_timeout(Duration::from_millis(10)).await;
        assert!(matches!(result, Err(IpcError::Timeout)));
    }

    #[tokio::test]
    async fn test_oneshot() {
        let (tx, rx) = oneshot::channel::<i32>();

        tx.send(42).unwrap();
        let value = rx.recv().await.unwrap();
        assert_eq!(value, 42);
    }

    #[tokio::test]
    async fn test_broadcast() {
        let (tx, mut rx1) = broadcast::channel::<String>(16);
        let mut rx2 = tx.subscribe();

        tx.send("Hello".to_string()).unwrap();

        let msg1 = rx1.recv().await.unwrap();
        let msg2 = rx2.recv().await.unwrap();

        assert_eq!(msg1, "Hello");
        assert_eq!(msg2, "Hello");
    }
}
