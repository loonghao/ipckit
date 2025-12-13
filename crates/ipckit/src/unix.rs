//! Unix-specific IPC utilities
//!
//! Provides Unix Domain Sockets and other Unix-specific IPC mechanisms.

use crate::error::{IpcError, Result};
use std::io::{Read, Write};
use std::os::unix::io::{FromRawFd, OwnedFd};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

/// Unix Domain Socket server
pub struct UnixSocketServer {
    listener: UnixListener,
    path: PathBuf,
}

/// Unix Domain Socket client connection
pub struct UnixSocketClient {
    stream: UnixStream,
}

/// Unix Domain Socket connection (from accept)
pub struct UnixSocketConnection {
    stream: UnixStream,
}

impl UnixSocketServer {
    /// Create a new Unix socket server at the given path
    pub fn bind<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        // Remove existing socket file if any
        let _ = std::fs::remove_file(&path);

        let listener = UnixListener::bind(&path).map_err(|e| match e.kind() {
            std::io::ErrorKind::PermissionDenied => {
                IpcError::PermissionDenied(path.display().to_string())
            }
            _ => IpcError::Io(e),
        })?;

        Ok(Self { listener, path })
    }

    /// Accept a new connection
    pub fn accept(&self) -> Result<UnixSocketConnection> {
        let (stream, _) = self.listener.accept()?;
        Ok(UnixSocketConnection { stream })
    }

    /// Set the socket to non-blocking mode
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<()> {
        self.listener.set_nonblocking(nonblocking)?;
        Ok(())
    }

    /// Get the socket path
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for UnixSocketServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

impl UnixSocketClient {
    /// Connect to a Unix socket server
    pub fn connect<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let stream = UnixStream::connect(path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => IpcError::NotFound(path.display().to_string()),
            std::io::ErrorKind::PermissionDenied => {
                IpcError::PermissionDenied(path.display().to_string())
            }
            std::io::ErrorKind::ConnectionRefused => {
                IpcError::NotFound(format!("Connection refused: {}", path.display()))
            }
            _ => IpcError::Io(e),
        })?;

        Ok(Self { stream })
    }

    /// Set the socket to non-blocking mode
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<()> {
        self.stream.set_nonblocking(nonblocking)?;
        Ok(())
    }
}

impl Read for UnixSocketClient {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.stream.read(buf)
    }
}

impl Write for UnixSocketClient {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.stream.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stream.flush()
    }
}

impl Read for UnixSocketConnection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.stream.read(buf)
    }
}

impl Write for UnixSocketConnection {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.stream.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stream.flush()
    }
}

/// Create a pair of connected Unix sockets
///
/// This is useful for parent-child process communication.
pub fn socketpair() -> Result<(OwnedFd, OwnedFd)> {
    let mut fds = [0i32; 2];
    let ret = unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, fds.as_mut_ptr()) };

    if ret < 0 {
        return Err(IpcError::Io(std::io::Error::last_os_error()));
    }

    Ok(unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) })
}

/// Signal handling utilities
pub mod signal {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    /// A flag that can be set by a signal handler
    pub struct SignalFlag {
        flag: Arc<AtomicBool>,
    }

    impl SignalFlag {
        /// Create a new signal flag
        pub fn new() -> Self {
            Self {
                flag: Arc::new(AtomicBool::new(false)),
            }
        }

        /// Check if the flag is set
        pub fn is_set(&self) -> bool {
            self.flag.load(Ordering::SeqCst)
        }

        /// Clear the flag
        pub fn clear(&self) {
            self.flag.store(false, Ordering::SeqCst);
        }

        /// Set the flag
        pub fn set(&self) {
            self.flag.store(true, Ordering::SeqCst);
        }

        /// Get a clone of the internal flag for use in signal handlers
        pub fn clone_flag(&self) -> Arc<AtomicBool> {
            self.flag.clone()
        }
    }

    impl Default for SignalFlag {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_unix_socket() {
        let path = format!("/tmp/test_socket_{}", std::process::id());

        let handle = thread::spawn({
            let path = path.clone();
            move || {
                let server = UnixSocketServer::bind(&path).unwrap();
                let mut conn = server.accept().unwrap();
                let mut buf = [0u8; 32];
                let n = conn.read(&mut buf).unwrap();
                assert_eq!(&buf[..n], b"Hello!");
                conn.write_all(b"World!").unwrap();
            }
        });

        thread::sleep(std::time::Duration::from_millis(100));

        let mut client = UnixSocketClient::connect(&path).unwrap();
        client.write_all(b"Hello!").unwrap();
        let mut buf = [0u8; 32];
        let n = client.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"World!");

        handle.join().unwrap();
    }

    #[test]
    fn test_socketpair() {
        let (fd1, fd2) = socketpair().unwrap();

        let handle = thread::spawn(move || {
            use std::os::unix::io::AsRawFd;
            let fd = fd2.as_raw_fd();
            let msg = b"Hello from child!";
            unsafe {
                libc::write(fd, msg.as_ptr() as *const _, msg.len());
            }
        });

        use std::os::unix::io::AsRawFd;
        let fd = fd1.as_raw_fd();
        let mut buf = [0u8; 32];
        let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut _, buf.len()) };
        assert!(n > 0);
        assert_eq!(&buf[..n as usize], b"Hello from child!");

        handle.join().unwrap();
    }
}
