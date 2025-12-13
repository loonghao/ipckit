//! Pipe implementations for IPC
//!
//! This module provides both anonymous pipes (for parent-child communication)
//! and named pipes (for unrelated process communication).

use crate::error::{IpcError, Result};
use std::io::{Read, Write};

/// Pipe reader end
pub struct PipeReader {
    #[cfg(unix)]
    inner: std::os::unix::io::OwnedFd,
    #[cfg(windows)]
    inner: windows::PipeHandle,
}

/// Pipe writer end
pub struct PipeWriter {
    #[cfg(unix)]
    inner: std::os::unix::io::OwnedFd,
    #[cfg(windows)]
    inner: windows::PipeHandle,
}

/// Anonymous pipe pair for parent-child process communication
pub struct AnonymousPipe {
    reader: PipeReader,
    writer: PipeWriter,
}

impl AnonymousPipe {
    /// Create a new anonymous pipe pair
    pub fn new() -> Result<Self> {
        #[cfg(unix)]
        {
            unix::create_anonymous_pipe()
        }
        #[cfg(windows)]
        {
            windows::create_anonymous_pipe()
        }
    }

    /// Split into reader and writer
    pub fn split(self) -> (PipeReader, PipeWriter) {
        (self.reader, self.writer)
    }

    /// Get a reference to the reader
    pub fn reader(&self) -> &PipeReader {
        &self.reader
    }

    /// Get a reference to the writer
    pub fn writer(&self) -> &PipeWriter {
        &self.writer
    }

    /// Get a mutable reference to the reader
    pub fn reader_mut(&mut self) -> &mut PipeReader {
        &mut self.reader
    }

    /// Get a mutable reference to the writer
    pub fn writer_mut(&mut self) -> &mut PipeWriter {
        &mut self.writer
    }
}

/// Named pipe for communication between unrelated processes
pub struct NamedPipe {
    name: String,
    #[cfg(unix)]
    inner: std::os::unix::io::OwnedFd,
    #[cfg(windows)]
    inner: windows::PipeHandle,
    is_server: bool,
}

impl NamedPipe {
    /// Create a new named pipe server
    ///
    /// On Unix, this creates a FIFO at the specified path.
    /// On Windows, this creates a named pipe with the given name.
    pub fn create(name: &str) -> Result<Self> {
        #[cfg(unix)]
        {
            unix::create_named_pipe(name)
        }
        #[cfg(windows)]
        {
            windows::create_named_pipe(name)
        }
    }

    /// Connect to an existing named pipe as a client
    pub fn connect(name: &str) -> Result<Self> {
        #[cfg(unix)]
        {
            unix::connect_named_pipe(name)
        }
        #[cfg(windows)]
        {
            windows::connect_named_pipe(name)
        }
    }

    /// Get the pipe name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Check if this is the server end
    pub fn is_server(&self) -> bool {
        self.is_server
    }

    /// Wait for a client to connect (server only)
    pub fn wait_for_client(&self) -> Result<()> {
        if !self.is_server {
            return Err(IpcError::InvalidState(
                "Only server can wait for clients".into(),
            ));
        }
        #[cfg(unix)]
        {
            // On Unix, the pipe is already connected after open
            Ok(())
        }
        #[cfg(windows)]
        {
            windows::wait_for_client(&self.inner)
        }
    }

    /// Disconnect the current client (server only, Windows)
    #[cfg(windows)]
    pub fn disconnect(&self) -> Result<()> {
        if !self.is_server {
            return Err(IpcError::InvalidState(
                "Only server can disconnect clients".into(),
            ));
        }
        windows::disconnect_named_pipe(&self.inner)
    }
}

#[cfg(unix)]
impl std::os::unix::io::AsRawFd for PipeReader {
    fn as_raw_fd(&self) -> std::os::unix::io::RawFd {
        use std::os::unix::io::AsRawFd;
        self.inner.as_raw_fd()
    }
}

#[cfg(unix)]
impl std::os::unix::io::AsRawFd for PipeWriter {
    fn as_raw_fd(&self) -> std::os::unix::io::RawFd {
        use std::os::unix::io::AsRawFd;
        self.inner.as_raw_fd()
    }
}

impl Read for PipeReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = self.inner.as_raw_fd();
            let ret = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut _, buf.len()) };
            if ret < 0 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(ret as usize)
            }
        }
        #[cfg(windows)]
        {
            windows::read_pipe(&self.inner, buf)
        }
    }
}

impl Write for PipeWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = self.inner.as_raw_fd();
            let ret = unsafe { libc::write(fd, buf.as_ptr() as *const _, buf.len()) };
            if ret < 0 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(ret as usize)
            }
        }
        #[cfg(windows)]
        {
            windows::write_pipe(&self.inner, buf)
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Read for NamedPipe {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = self.inner.as_raw_fd();
            let ret = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut _, buf.len()) };
            if ret < 0 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(ret as usize)
            }
        }
        #[cfg(windows)]
        {
            windows::read_pipe(&self.inner, buf)
        }
    }
}

impl Write for NamedPipe {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = self.inner.as_raw_fd();
            let ret = unsafe { libc::write(fd, buf.as_ptr() as *const _, buf.len()) };
            if ret < 0 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(ret as usize)
            }
        }
        #[cfg(windows)]
        {
            windows::write_pipe(&self.inner, buf)
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// Platform-specific implementations
#[cfg(unix)]
mod unix {
    use super::*;
    use std::ffi::CString;
    use std::os::unix::io::{FromRawFd, OwnedFd};

    pub fn create_anonymous_pipe() -> Result<AnonymousPipe> {
        let mut fds = [0i32; 2];
        let ret = unsafe { libc::pipe(fds.as_mut_ptr()) };
        if ret < 0 {
            return Err(IpcError::Io(std::io::Error::last_os_error()));
        }

        let reader = PipeReader {
            inner: unsafe { OwnedFd::from_raw_fd(fds[0]) },
        };
        let writer = PipeWriter {
            inner: unsafe { OwnedFd::from_raw_fd(fds[1]) },
        };

        Ok(AnonymousPipe { reader, writer })
    }

    pub fn create_named_pipe(name: &str) -> Result<NamedPipe> {
        let path = if name.starts_with('/') {
            name.to_string()
        } else {
            format!("/tmp/{}", name)
        };

        let c_path = CString::new(path.clone())
            .map_err(|_| IpcError::InvalidName("Invalid pipe name".into()))?;

        // Remove existing FIFO if any
        unsafe { libc::unlink(c_path.as_ptr()) };

        // Create FIFO with rw-rw-rw- permissions
        let ret = unsafe { libc::mkfifo(c_path.as_ptr(), 0o666) };
        if ret < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(IpcError::Io(err));
            }
        }

        // Open for read-write (non-blocking initially to avoid deadlock)
        let fd = unsafe { libc::open(c_path.as_ptr(), libc::O_RDWR | libc::O_NONBLOCK) };
        if fd < 0 {
            return Err(IpcError::Io(std::io::Error::last_os_error()));
        }

        // Set back to blocking mode
        unsafe {
            let flags = libc::fcntl(fd, libc::F_GETFL);
            libc::fcntl(fd, libc::F_SETFL, flags & !libc::O_NONBLOCK);
        }

        Ok(NamedPipe {
            name: path,
            inner: unsafe { OwnedFd::from_raw_fd(fd) },
            is_server: true,
        })
    }

    pub fn connect_named_pipe(name: &str) -> Result<NamedPipe> {
        let path = if name.starts_with('/') {
            name.to_string()
        } else {
            format!("/tmp/{}", name)
        };

        let c_path = CString::new(path.clone())
            .map_err(|_| IpcError::InvalidName("Invalid pipe name".into()))?;

        let fd = unsafe { libc::open(c_path.as_ptr(), libc::O_RDWR) };
        if fd < 0 {
            let err = std::io::Error::last_os_error();
            return Err(match err.kind() {
                std::io::ErrorKind::NotFound => IpcError::NotFound(path),
                std::io::ErrorKind::PermissionDenied => IpcError::PermissionDenied(path),
                _ => IpcError::Io(err),
            });
        }

        Ok(NamedPipe {
            name: path,
            inner: unsafe { OwnedFd::from_raw_fd(fd) },
            is_server: false,
        })
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;
    use windows_sys::Win32::Foundation::*;
    use windows_sys::Win32::Storage::FileSystem::*;
    use windows_sys::Win32::System::Pipes::*;

    pub struct PipeHandle {
        handle: HANDLE,
    }

    impl PipeHandle {
        pub fn new(handle: HANDLE) -> Self {
            Self { handle }
        }

        pub fn as_raw(&self) -> HANDLE {
            self.handle
        }
    }

    impl Drop for PipeHandle {
        fn drop(&mut self) {
            if self.handle != INVALID_HANDLE_VALUE {
                unsafe { CloseHandle(self.handle) };
            }
        }
    }

    // Make PipeHandle Send + Sync
    unsafe impl Send for PipeHandle {}
    unsafe impl Sync for PipeHandle {}

    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(Some(0)).collect()
    }

    pub fn create_anonymous_pipe() -> Result<AnonymousPipe> {
        let mut read_handle: HANDLE = INVALID_HANDLE_VALUE;
        let mut write_handle: HANDLE = INVALID_HANDLE_VALUE;

        let ret = unsafe { CreatePipe(&mut read_handle, &mut write_handle, ptr::null(), 0) };

        if ret == 0 {
            return Err(IpcError::Io(std::io::Error::last_os_error()));
        }

        Ok(AnonymousPipe {
            reader: PipeReader {
                inner: PipeHandle::new(read_handle),
            },
            writer: PipeWriter {
                inner: PipeHandle::new(write_handle),
            },
        })
    }

    pub fn create_named_pipe(name: &str) -> Result<NamedPipe> {
        let pipe_name = if name.starts_with(r"\\.\pipe\") {
            name.to_string()
        } else {
            format!(r"\\.\pipe\{}", name)
        };

        let wide_name = to_wide(&pipe_name);

        let handle = unsafe {
            CreateNamedPipeW(
                wide_name.as_ptr(),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                PIPE_UNLIMITED_INSTANCES,
                4096,
                4096,
                0,
                ptr::null(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            return Err(IpcError::Io(std::io::Error::last_os_error()));
        }

        Ok(NamedPipe {
            name: pipe_name,
            inner: PipeHandle::new(handle),
            is_server: true,
        })
    }

    pub fn connect_named_pipe(name: &str) -> Result<NamedPipe> {
        let pipe_name = if name.starts_with(r"\\.\pipe\") {
            name.to_string()
        } else {
            format!(r"\\.\pipe\{}", name)
        };

        let wide_name = to_wide(&pipe_name);

        let handle = unsafe {
            CreateFileW(
                wide_name.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                ptr::null(),
                OPEN_EXISTING,
                0,
                INVALID_HANDLE_VALUE,
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            let err = std::io::Error::last_os_error();
            return Err(match err.raw_os_error() {
                Some(2) => IpcError::NotFound(pipe_name), // ERROR_FILE_NOT_FOUND
                Some(5) => IpcError::PermissionDenied(pipe_name), // ERROR_ACCESS_DENIED
                _ => IpcError::Io(err),
            });
        }

        Ok(NamedPipe {
            name: pipe_name,
            inner: PipeHandle::new(handle),
            is_server: false,
        })
    }

    pub fn wait_for_client(handle: &PipeHandle) -> Result<()> {
        let ret = unsafe { ConnectNamedPipe(handle.as_raw(), ptr::null_mut()) };
        if ret == 0 {
            let err = std::io::Error::last_os_error();
            // ERROR_PIPE_CONNECTED means client is already connected
            if err.raw_os_error() != Some(535) {
                return Err(IpcError::Io(err));
            }
        }
        Ok(())
    }

    pub fn disconnect_named_pipe(handle: &PipeHandle) -> Result<()> {
        let ret = unsafe { DisconnectNamedPipe(handle.as_raw()) };
        if ret == 0 {
            return Err(IpcError::Io(std::io::Error::last_os_error()));
        }
        Ok(())
    }

    pub fn read_pipe(handle: &PipeHandle, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut bytes_read: u32 = 0;
        let ret = unsafe {
            ReadFile(
                handle.as_raw(),
                buf.as_mut_ptr() as *mut _,
                buf.len() as u32,
                &mut bytes_read,
                ptr::null_mut(),
            )
        };
        if ret == 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(bytes_read as usize)
        }
    }

    pub fn write_pipe(handle: &PipeHandle, buf: &[u8]) -> std::io::Result<usize> {
        let mut bytes_written: u32 = 0;
        let ret = unsafe {
            WriteFile(
                handle.as_raw(),
                buf.as_ptr() as *const _,
                buf.len() as u32,
                &mut bytes_written,
                ptr::null_mut(),
            )
        };
        if ret == 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(bytes_written as usize)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anonymous_pipe() {
        let pipe = AnonymousPipe::new().unwrap();
        let (mut reader, mut writer) = pipe.split();

        let msg = b"Hello, IPC!";
        writer.write_all(msg).unwrap();

        let mut buf = [0u8; 32];
        let n = reader.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], msg);
    }
}
