//! Local Socket implementation for IPC
//!
//! This module provides a cross-platform local socket abstraction for IPC.
//! When the `backend-interprocess` feature is enabled, it uses the `interprocess` crate
//! for a more robust implementation. Otherwise, it falls back to the native implementation.
//!
//! # Features
//! - Unix Domain Sockets on Unix systems
//! - Named Pipes on Windows
//! - Server/Client architecture
//! - Async support (with `async` feature)

use crate::error::Result;
use std::io::{Read, Write};

// ============================================================================
// Backend: interprocess
// ============================================================================

#[cfg(feature = "backend-interprocess")]
mod interprocess_backend {
    use super::*;
    use crate::error::IpcError;
    use interprocess::local_socket::{
        prelude::*, GenericFilePath, GenericNamespaced, ListenerOptions, Stream, ToFsName, ToNsName,
    };

    /// A local socket listener that accepts incoming connections.
    pub struct LocalSocketListener {
        listener: interprocess::local_socket::Listener,
        name: String,
    }

    /// A local socket stream for bidirectional communication.
    pub struct LocalSocketStream {
        inner: Stream,
        name: String,
    }

    impl LocalSocketListener {
        /// Create a new local socket listener bound to the given name.
        pub fn bind(name: &str) -> Result<Self> {
            let socket_name = get_socket_name(name)?;

            let listener = ListenerOptions::new()
                .name(socket_name)
                .create_sync()
                .map_err(|e| IpcError::Io(std::io::Error::other(e)))?;

            Ok(Self {
                listener,
                name: name.to_string(),
            })
        }

        /// Accept a new incoming connection.
        pub fn accept(&self) -> Result<LocalSocketStream> {
            let stream = self
                .listener
                .accept()
                .map_err(|e| IpcError::Io(std::io::Error::other(e)))?;

            Ok(LocalSocketStream {
                inner: stream,
                name: self.name.clone(),
            })
        }

        /// Get the name of this listener.
        pub fn name(&self) -> &str {
            &self.name
        }

        /// Returns an iterator over incoming connections.
        pub fn incoming(&self) -> impl Iterator<Item = Result<LocalSocketStream>> + '_ {
            std::iter::from_fn(move || Some(self.accept()))
        }
    }

    impl LocalSocketStream {
        /// Connect to a local socket server.
        pub fn connect(name: &str) -> Result<Self> {
            let socket_name = get_socket_name(name)?;

            let stream =
                Stream::connect(socket_name).map_err(|e| IpcError::Io(std::io::Error::other(e)))?;

            Ok(Self {
                inner: stream,
                name: name.to_string(),
            })
        }

        /// Get the name of this stream.
        pub fn name(&self) -> &str {
            &self.name
        }
    }

    impl Read for LocalSocketStream {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.inner.read(buf)
        }
    }

    impl Write for LocalSocketStream {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.inner.write(buf)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.inner.flush()
        }
    }

    /// Get the appropriate socket name for the current platform.
    fn get_socket_name(name: &str) -> Result<interprocess::local_socket::Name<'static>> {
        // Try namespaced name first (works on Linux with abstract sockets and Windows)
        if let Ok(ns_name) = name.to_string().to_ns_name::<GenericNamespaced>() {
            return Ok(ns_name);
        }

        // Fall back to filesystem path
        let path = if cfg!(unix) {
            if name.starts_with('/') {
                name.to_string()
            } else {
                format!("/tmp/{}.sock", name)
            }
        } else {
            // Windows named pipe
            if name.starts_with(r"\\.\pipe\") {
                name.to_string()
            } else {
                format!(r"\\.\pipe\{}", name)
            }
        };

        path.to_fs_name::<GenericFilePath>()
            .map_err(|e| IpcError::Io(std::io::Error::other(e)))
    }
}

#[cfg(feature = "backend-interprocess")]
pub use interprocess_backend::{LocalSocketListener, LocalSocketStream};

// ============================================================================
// Backend: Native (fallback)
// ============================================================================

#[cfg(not(feature = "backend-interprocess"))]
mod native_backend {
    use super::*;
    #[cfg(unix)]
    use crate::error::IpcError;

    #[cfg(unix)]
    use std::os::unix::net::{UnixListener, UnixStream};

    /// A local socket listener that accepts incoming connections.
    pub struct LocalSocketListener {
        #[cfg(unix)]
        listener: UnixListener,
        #[cfg(unix)]
        path: String,
        #[cfg(windows)]
        pipe_name: String,
        name: String,
    }

    /// A local socket stream for bidirectional communication.
    pub struct LocalSocketStream {
        #[cfg(unix)]
        stream: UnixStream,
        #[cfg(windows)]
        handle: crate::windows::PipeHandle,
        name: String,
    }

    impl LocalSocketListener {
        /// Create a new local socket listener bound to the given name.
        pub fn bind(name: &str) -> Result<Self> {
            #[cfg(unix)]
            {
                let path = if name.starts_with('/') {
                    name.to_string()
                } else {
                    format!("/tmp/{}.sock", name)
                };

                // Remove existing socket if any
                let _ = std::fs::remove_file(&path);

                let listener = UnixListener::bind(&path).map_err(|e| match e.kind() {
                    std::io::ErrorKind::PermissionDenied => {
                        IpcError::PermissionDenied(path.clone())
                    }
                    _ => IpcError::Io(e),
                })?;

                Ok(Self {
                    listener,
                    path,
                    name: name.to_string(),
                })
            }

            #[cfg(windows)]
            {
                let pipe_name = if name.starts_with(r"\\.\pipe\") {
                    name.to_string()
                } else {
                    format!(r"\\.\pipe\{}", name)
                };

                Ok(Self {
                    pipe_name,
                    name: name.to_string(),
                })
            }
        }

        /// Accept a new incoming connection.
        pub fn accept(&self) -> Result<LocalSocketStream> {
            #[cfg(unix)]
            {
                let (stream, _) = self.listener.accept()?;
                Ok(LocalSocketStream {
                    stream,
                    name: self.name.clone(),
                })
            }

            #[cfg(windows)]
            {
                use crate::windows;
                let handle = windows::create_named_pipe_for_server(&self.pipe_name)?;
                windows::wait_for_client_handle(&handle)?;
                Ok(LocalSocketStream {
                    handle,
                    name: self.name.clone(),
                })
            }
        }

        /// Get the name of this listener.
        pub fn name(&self) -> &str {
            &self.name
        }

        /// Returns an iterator over incoming connections.
        pub fn incoming(&self) -> impl Iterator<Item = Result<LocalSocketStream>> + '_ {
            std::iter::from_fn(move || Some(self.accept()))
        }
    }

    #[cfg(unix)]
    impl Drop for LocalSocketListener {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    impl LocalSocketStream {
        /// Connect to a local socket server.
        pub fn connect(name: &str) -> Result<Self> {
            #[cfg(unix)]
            {
                let path = if name.starts_with('/') {
                    name.to_string()
                } else {
                    format!("/tmp/{}.sock", name)
                };

                let stream = UnixStream::connect(&path).map_err(|e| match e.kind() {
                    std::io::ErrorKind::NotFound => IpcError::NotFound(path.clone()),
                    std::io::ErrorKind::PermissionDenied => {
                        IpcError::PermissionDenied(path.clone())
                    }
                    std::io::ErrorKind::ConnectionRefused => {
                        IpcError::NotFound(format!("Connection refused: {}", path))
                    }
                    _ => IpcError::Io(e),
                })?;

                Ok(Self {
                    stream,
                    name: name.to_string(),
                })
            }

            #[cfg(windows)]
            {
                use crate::windows;
                let pipe_name = if name.starts_with(r"\\.\pipe\") {
                    name.to_string()
                } else {
                    format!(r"\\.\pipe\{}", name)
                };

                let handle = windows::connect_to_named_pipe(&pipe_name)?;
                Ok(Self {
                    handle,
                    name: name.to_string(),
                })
            }
        }

        /// Get the name of this stream.
        pub fn name(&self) -> &str {
            &self.name
        }
    }

    impl Read for LocalSocketStream {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            #[cfg(unix)]
            {
                self.stream.read(buf)
            }
            #[cfg(windows)]
            {
                crate::windows::read_pipe(&self.handle, buf)
            }
        }
    }

    impl Write for LocalSocketStream {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            #[cfg(unix)]
            {
                self.stream.write(buf)
            }
            #[cfg(windows)]
            {
                crate::windows::write_pipe(&self.handle, buf)
            }
        }

        fn flush(&mut self) -> std::io::Result<()> {
            #[cfg(unix)]
            {
                self.stream.flush()
            }
            #[cfg(windows)]
            {
                Ok(())
            }
        }
    }
}

#[cfg(not(feature = "backend-interprocess"))]
pub use native_backend::{LocalSocketListener, LocalSocketStream};

// ============================================================================
// Async support
// ============================================================================

#[cfg(all(feature = "async", feature = "backend-interprocess"))]
pub mod async_socket {
    //! Async local socket support using tokio.

    use super::*;
    use crate::error::IpcError;
    use interprocess::local_socket::{
        tokio::prelude::*, GenericFilePath, GenericNamespaced, ListenerOptions, ToFsName, ToNsName,
    };
    use tokio::io::{AsyncRead, AsyncWrite};

    /// Async local socket listener.
    pub struct AsyncLocalSocketListener {
        inner: interprocess::local_socket::tokio::Listener,
        name: String,
    }

    /// Async local socket stream.
    pub struct AsyncLocalSocketStream {
        inner: interprocess::local_socket::tokio::Stream,
        name: String,
    }

    impl AsyncLocalSocketListener {
        /// Create a new async local socket listener.
        pub async fn bind(name: &str) -> Result<Self> {
            let socket_name = get_async_socket_name(name)?;

            let listener = ListenerOptions::new()
                .name(socket_name)
                .create_tokio()
                .map_err(|e| IpcError::Io(std::io::Error::other(e)))?;

            Ok(Self {
                inner: listener,
                name: name.to_string(),
            })
        }

        /// Accept a new incoming connection asynchronously.
        pub async fn accept(&self) -> Result<AsyncLocalSocketStream> {
            let stream = self
                .inner
                .accept()
                .await
                .map_err(|e| IpcError::Io(std::io::Error::other(e)))?;

            Ok(AsyncLocalSocketStream {
                inner: stream,
                name: self.name.clone(),
            })
        }

        /// Get the name of this listener.
        pub fn name(&self) -> &str {
            &self.name
        }
    }

    impl AsyncLocalSocketStream {
        /// Connect to a local socket server asynchronously.
        pub async fn connect(name: &str) -> Result<Self> {
            let socket_name = get_async_socket_name(name)?;

            let stream = interprocess::local_socket::tokio::Stream::connect(socket_name)
                .await
                .map_err(|e| IpcError::Io(std::io::Error::other(e)))?;

            Ok(Self {
                inner: stream,
                name: name.to_string(),
            })
        }

        /// Get the name of this stream.
        pub fn name(&self) -> &str {
            &self.name
        }

        /// Split into read and write halves.
        pub fn into_split(
            self,
        ) -> (
            tokio::io::ReadHalf<interprocess::local_socket::tokio::Stream>,
            tokio::io::WriteHalf<interprocess::local_socket::tokio::Stream>,
        ) {
            tokio::io::split(self.inner)
        }
    }

    impl AsyncRead for AsyncLocalSocketStream {
        fn poll_read(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::pin::Pin::new(&mut self.inner).poll_read(cx, buf)
        }
    }

    impl AsyncWrite for AsyncLocalSocketStream {
        fn poll_write(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &[u8],
        ) -> std::task::Poll<std::io::Result<usize>> {
            std::pin::Pin::new(&mut self.inner).poll_write(cx, buf)
        }

        fn poll_flush(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::pin::Pin::new(&mut self.inner).poll_flush(cx)
        }

        fn poll_shutdown(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::pin::Pin::new(&mut self.inner).poll_shutdown(cx)
        }
    }

    fn get_async_socket_name(name: &str) -> Result<interprocess::local_socket::Name<'static>> {
        if let Ok(ns_name) = name.to_string().to_ns_name::<GenericNamespaced>() {
            return Ok(ns_name);
        }

        let path = if cfg!(unix) {
            if name.starts_with('/') {
                name.to_string()
            } else {
                format!("/tmp/{}.sock", name)
            }
        } else if name.starts_with(r"\\.\pipe\") {
            name.to_string()
        } else {
            format!(r"\\.\pipe\{}", name)
        };

        path.to_fs_name::<GenericFilePath>()
            .map_err(|e| IpcError::Io(std::io::Error::other(e)))
    }
}

#[cfg(all(feature = "async", feature = "backend-interprocess"))]
pub use async_socket::{AsyncLocalSocketListener, AsyncLocalSocketStream};

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_local_socket_communication() {
        let server_name = format!("test_socket_{}", std::process::id());

        // Create server in a separate thread
        let server_name_clone = server_name.clone();
        let server_thread = thread::spawn(move || {
            let listener = LocalSocketListener::bind(&server_name_clone).unwrap();
            let mut stream = listener.accept().unwrap();

            let mut buf = [0u8; 32];
            let n = stream.read(&mut buf).unwrap();
            assert_eq!(&buf[..n], b"Hello, Server!");

            stream.write_all(b"Hello, Client!").unwrap();
        });

        // Give server time to start
        thread::sleep(std::time::Duration::from_millis(100));

        // Connect as client
        let mut client = LocalSocketStream::connect(&server_name).unwrap();
        client.write_all(b"Hello, Server!").unwrap();

        let mut buf = [0u8; 32];
        let n = client.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"Hello, Client!");

        server_thread.join().unwrap();
    }
}
