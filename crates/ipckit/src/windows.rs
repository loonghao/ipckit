//! Windows-specific IPC utilities
//!
//! Provides Named Pipes and other Windows-specific IPC mechanisms.

use crate::error::{IpcError, Result};
use std::ffi::OsStr;
use std::io::{Read, Write};
use std::os::windows::ffi::OsStrExt;
use std::ptr;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Storage::FileSystem::*;
use windows_sys::Win32::System::Pipes::*;

/// Windows Named Pipe handle wrapper
pub struct PipeHandle {
    handle: HANDLE,
}

impl PipeHandle {
    /// Create a new pipe handle
    pub fn new(handle: HANDLE) -> Self {
        Self { handle }
    }

    /// Get the raw handle
    pub fn as_raw(&self) -> HANDLE {
        self.handle
    }

    /// Check if the handle is valid
    pub fn is_valid(&self) -> bool {
        self.handle != INVALID_HANDLE_VALUE
    }
}

impl Drop for PipeHandle {
    fn drop(&mut self) {
        if self.is_valid() {
            unsafe { CloseHandle(self.handle) };
        }
    }
}

unsafe impl Send for PipeHandle {}
unsafe impl Sync for PipeHandle {}

/// Windows Named Pipe server
pub struct NamedPipeServer {
    handle: PipeHandle,
    name: String,
}

/// Windows Named Pipe client
pub struct NamedPipeClient {
    handle: PipeHandle,
    name: String,
}

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

fn pipe_name(name: &str) -> String {
    if name.starts_with(r"\\.\pipe\") {
        name.to_string()
    } else {
        format!(r"\\.\pipe\{}", name)
    }
}

impl NamedPipeServer {
    /// Create a new named pipe server
    ///
    /// # Arguments
    /// * `name` - The pipe name (will be prefixed with `\\.\pipe\` if not already)
    /// * `max_instances` - Maximum number of instances (use 0 for unlimited)
    pub fn create(name: &str, max_instances: u32) -> Result<Self> {
        let pipe_name = pipe_name(name);
        let wide_name = to_wide(&pipe_name);

        let instances = if max_instances == 0 {
            PIPE_UNLIMITED_INSTANCES
        } else {
            max_instances
        };

        let handle = unsafe {
            CreateNamedPipeW(
                wide_name.as_ptr(),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                instances,
                4096,  // Output buffer size
                4096,  // Input buffer size
                0,     // Default timeout
                ptr::null(),
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            return Err(IpcError::Io(std::io::Error::last_os_error()));
        }

        Ok(Self {
            handle: PipeHandle::new(handle),
            name: pipe_name,
        })
    }

    /// Wait for a client to connect
    pub fn wait_for_connection(&self) -> Result<()> {
        let ret = unsafe { ConnectNamedPipe(self.handle.as_raw(), ptr::null_mut()) };

        if ret == 0 {
            let err = std::io::Error::last_os_error();
            // ERROR_PIPE_CONNECTED (535) means client already connected
            if err.raw_os_error() != Some(535) {
                return Err(IpcError::Io(err));
            }
        }

        Ok(())
    }

    /// Disconnect the current client
    pub fn disconnect(&self) -> Result<()> {
        let ret = unsafe { DisconnectNamedPipe(self.handle.as_raw()) };
        if ret == 0 {
            return Err(IpcError::Io(std::io::Error::last_os_error()));
        }
        Ok(())
    }

    /// Get the pipe name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Flush the pipe buffers
    pub fn flush(&self) -> Result<()> {
        let ret = unsafe { FlushFileBuffers(self.handle.as_raw()) };
        if ret == 0 {
            return Err(IpcError::Io(std::io::Error::last_os_error()));
        }
        Ok(())
    }
}

impl Read for NamedPipeServer {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut bytes_read: u32 = 0;
        let ret = unsafe {
            ReadFile(
                self.handle.as_raw(),
                buf.as_mut_ptr() as *mut _,
                buf.len() as u32,
                &mut bytes_read,
                ptr::null_mut(),
            )
        };

        if ret == 0 {
            let err = std::io::Error::last_os_error();
            // ERROR_BROKEN_PIPE means the client disconnected
            if err.raw_os_error() == Some(109) {
                return Ok(0);
            }
            return Err(err);
        }

        Ok(bytes_read as usize)
    }
}

impl Write for NamedPipeServer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut bytes_written: u32 = 0;
        let ret = unsafe {
            WriteFile(
                self.handle.as_raw(),
                buf.as_ptr() as *const _,
                buf.len() as u32,
                &mut bytes_written,
                ptr::null_mut(),
            )
        };

        if ret == 0 {
            return Err(std::io::Error::last_os_error());
        }

        Ok(bytes_written as usize)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        NamedPipeServer::flush(self).map_err(|e| match e {
            IpcError::Io(e) => e,
            _ => std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
        })
    }
}

impl NamedPipeClient {
    /// Connect to an existing named pipe
    pub fn connect(name: &str) -> Result<Self> {
        let pipe_name = pipe_name(name);
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
                Some(2) => IpcError::NotFound(pipe_name),   // ERROR_FILE_NOT_FOUND
                Some(5) => IpcError::PermissionDenied(pipe_name), // ERROR_ACCESS_DENIED
                Some(231) => IpcError::InvalidState("All pipe instances are busy".into()), // ERROR_PIPE_BUSY
                _ => IpcError::Io(err),
            });
        }

        Ok(Self {
            handle: PipeHandle::new(handle),
            name: pipe_name,
        })
    }

    /// Connect with timeout (in milliseconds)
    pub fn connect_with_timeout(name: &str, timeout_ms: u32) -> Result<Self> {
        let pipe_name = pipe_name(name);
        let wide_name = to_wide(&pipe_name);

        // Wait for the pipe to become available
        let ret = unsafe { WaitNamedPipeW(wide_name.as_ptr(), timeout_ms) };

        if ret == 0 {
            let err = std::io::Error::last_os_error();
            return Err(match err.raw_os_error() {
                Some(2) => IpcError::NotFound(pipe_name),
                Some(121) => IpcError::Timeout, // ERROR_SEM_TIMEOUT
                _ => IpcError::Io(err),
            });
        }

        Self::connect(name)
    }

    /// Get the pipe name
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Read for NamedPipeClient {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut bytes_read: u32 = 0;
        let ret = unsafe {
            ReadFile(
                self.handle.as_raw(),
                buf.as_mut_ptr() as *mut _,
                buf.len() as u32,
                &mut bytes_read,
                ptr::null_mut(),
            )
        };

        if ret == 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(109) {
                return Ok(0);
            }
            return Err(err);
        }

        Ok(bytes_read as usize)
    }
}

impl Write for NamedPipeClient {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut bytes_written: u32 = 0;
        let ret = unsafe {
            WriteFile(
                self.handle.as_raw(),
                buf.as_ptr() as *const _,
                buf.len() as u32,
                &mut bytes_written,
                ptr::null_mut(),
            )
        };

        if ret == 0 {
            return Err(std::io::Error::last_os_error());
        }

        Ok(bytes_written as usize)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let ret = unsafe { FlushFileBuffers(self.handle.as_raw()) };
        if ret == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
}

// Note: Mailslot functionality removed for simplicity
// Use Named Pipes for bidirectional communication instead

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_named_pipe() {
        let name = format!("test_pipe_{}", std::process::id());

        let handle = thread::spawn({
            let name = name.clone();
            move || {
                let mut server = NamedPipeServer::create(&name, 1).unwrap();
                server.wait_for_connection().unwrap();
                let mut buf = [0u8; 32];
                let n = server.read(&mut buf).unwrap();
                assert_eq!(&buf[..n], b"Hello!");
                server.write_all(b"World!").unwrap();
            }
        });

        thread::sleep(std::time::Duration::from_millis(100));

        let mut client = NamedPipeClient::connect(&name).unwrap();
        client.write_all(b"Hello!").unwrap();
        let mut buf = [0u8; 32];
        let n = client.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"World!");

        handle.join().unwrap();
    }
}
