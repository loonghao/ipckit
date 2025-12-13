//! Shared Memory implementation for IPC
//!
//! Provides memory-mapped shared memory regions for fast data exchange between processes.

use crate::error::{IpcError, Result};
use std::ptr::NonNull;

/// Shared memory region for inter-process communication
pub struct SharedMemory {
    name: String,
    ptr: NonNull<u8>,
    size: usize,
    is_owner: bool,
    #[cfg(unix)]
    fd: std::os::unix::io::RawFd,
    #[cfg(windows)]
    handle: windows_sys::Win32::Foundation::HANDLE,
}

// Safety: SharedMemory uses proper synchronization
unsafe impl Send for SharedMemory {}
unsafe impl Sync for SharedMemory {}

impl SharedMemory {
    /// Create a new shared memory region with the given name and size
    ///
    /// The name should be unique across the system. On Unix, it will be prefixed
    /// with `/` if not already. On Windows, it will be used as-is.
    pub fn create(name: &str, size: usize) -> Result<Self> {
        if size == 0 {
            return Err(IpcError::InvalidName("Size must be greater than 0".into()));
        }

        #[cfg(unix)]
        {
            unix::create_shm(name, size)
        }
        #[cfg(windows)]
        {
            windows::create_shm(name, size)
        }
    }

    /// Open an existing shared memory region
    pub fn open(name: &str) -> Result<Self> {
        #[cfg(unix)]
        {
            unix::open_shm(name)
        }
        #[cfg(windows)]
        {
            windows::open_shm(name)
        }
    }

    /// Get the name of the shared memory region
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the size of the shared memory region
    pub fn size(&self) -> usize {
        self.size
    }

    /// Check if this instance is the owner (creator) of the shared memory
    pub fn is_owner(&self) -> bool {
        self.is_owner
    }

    /// Get a pointer to the shared memory
    ///
    /// # Safety
    /// The caller must ensure proper synchronization when accessing the memory.
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr.as_ptr()
    }

    /// Get a mutable pointer to the shared memory
    ///
    /// # Safety
    /// The caller must ensure proper synchronization when accessing the memory.
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr.as_ptr()
    }

    /// Get a slice view of the shared memory
    ///
    /// # Safety
    /// The caller must ensure no other process is writing to this region.
    pub unsafe fn as_slice(&self) -> &[u8] {
        std::slice::from_raw_parts(self.ptr.as_ptr(), self.size)
    }

    /// Get a mutable slice view of the shared memory
    ///
    /// # Safety
    /// The caller must ensure exclusive access to this region.
    pub unsafe fn as_mut_slice(&mut self) -> &mut [u8] {
        std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.size)
    }

    /// Write data to the shared memory at the given offset
    ///
    /// Returns error if offset + data.len() exceeds the size.
    pub fn write(&mut self, offset: usize, data: &[u8]) -> Result<()> {
        if offset + data.len() > self.size {
            return Err(IpcError::BufferTooSmall {
                needed: offset + data.len(),
                got: self.size,
            });
        }

        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), self.ptr.as_ptr().add(offset), data.len());
        }
        Ok(())
    }

    /// Read data from the shared memory at the given offset
    ///
    /// Returns error if offset + len exceeds the size.
    pub fn read(&self, offset: usize, len: usize) -> Result<Vec<u8>> {
        if offset + len > self.size {
            return Err(IpcError::BufferTooSmall {
                needed: offset + len,
                got: self.size,
            });
        }

        let mut buf = vec![0u8; len];
        unsafe {
            std::ptr::copy_nonoverlapping(self.ptr.as_ptr().add(offset), buf.as_mut_ptr(), len);
        }
        Ok(buf)
    }

    /// Read data into an existing buffer
    pub fn read_into(&self, offset: usize, buf: &mut [u8]) -> Result<()> {
        if offset + buf.len() > self.size {
            return Err(IpcError::BufferTooSmall {
                needed: offset + buf.len(),
                got: self.size,
            });
        }

        unsafe {
            std::ptr::copy_nonoverlapping(
                self.ptr.as_ptr().add(offset),
                buf.as_mut_ptr(),
                buf.len(),
            );
        }
        Ok(())
    }
}

impl Drop for SharedMemory {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            unsafe {
                libc::munmap(self.ptr.as_ptr() as *mut _, self.size);
                libc::close(self.fd);
                if self.is_owner {
                    let c_name = std::ffi::CString::new(self.name.clone()).unwrap();
                    libc::shm_unlink(c_name.as_ptr());
                }
            }
        }
        #[cfg(windows)]
        {
            unsafe {
                use windows_sys::Win32::System::Memory::MEMORY_MAPPED_VIEW_ADDRESS;
                let addr = MEMORY_MAPPED_VIEW_ADDRESS {
                    Value: self.ptr.as_ptr() as *mut _,
                };
                windows_sys::Win32::System::Memory::UnmapViewOfFile(addr);
                windows_sys::Win32::Foundation::CloseHandle(self.handle);
            }
        }
    }
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::ffi::CString;

    pub fn create_shm(name: &str, size: usize) -> Result<SharedMemory> {
        let shm_name = if name.starts_with('/') {
            name.to_string()
        } else {
            format!("/{}", name)
        };

        let c_name = CString::new(shm_name.clone())
            .map_err(|_| IpcError::InvalidName("Invalid shared memory name".into()))?;

        // Create shared memory object
        let fd = unsafe {
            libc::shm_open(
                c_name.as_ptr(),
                libc::O_CREAT | libc::O_RDWR | libc::O_EXCL,
                0o666,
            )
        };

        if fd < 0 {
            let err = std::io::Error::last_os_error();
            return Err(match err.kind() {
                std::io::ErrorKind::AlreadyExists => IpcError::AlreadyExists(shm_name),
                std::io::ErrorKind::PermissionDenied => IpcError::PermissionDenied(shm_name),
                _ => IpcError::Io(err),
            });
        }

        // Set size
        if unsafe { libc::ftruncate(fd, size as libc::off_t) } < 0 {
            unsafe {
                libc::close(fd);
                libc::shm_unlink(c_name.as_ptr());
            }
            return Err(IpcError::Io(std::io::Error::last_os_error()));
        }

        // Map memory
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };

        if ptr == libc::MAP_FAILED {
            unsafe {
                libc::close(fd);
                libc::shm_unlink(c_name.as_ptr());
            }
            return Err(IpcError::Io(std::io::Error::last_os_error()));
        }

        Ok(SharedMemory {
            name: shm_name,
            ptr: NonNull::new(ptr as *mut u8).unwrap(),
            size,
            is_owner: true,
            fd,
        })
    }

    pub fn open_shm(name: &str) -> Result<SharedMemory> {
        let shm_name = if name.starts_with('/') {
            name.to_string()
        } else {
            format!("/{}", name)
        };

        let c_name = CString::new(shm_name.clone())
            .map_err(|_| IpcError::InvalidName("Invalid shared memory name".into()))?;

        // Open existing shared memory object
        let fd = unsafe { libc::shm_open(c_name.as_ptr(), libc::O_RDWR, 0) };

        if fd < 0 {
            let err = std::io::Error::last_os_error();
            return Err(match err.kind() {
                std::io::ErrorKind::NotFound => IpcError::NotFound(shm_name),
                std::io::ErrorKind::PermissionDenied => IpcError::PermissionDenied(shm_name),
                _ => IpcError::Io(err),
            });
        }

        // Get size
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        if unsafe { libc::fstat(fd, &mut stat) } < 0 {
            unsafe { libc::close(fd) };
            return Err(IpcError::Io(std::io::Error::last_os_error()));
        }
        let size = stat.st_size as usize;

        // Map memory
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };

        if ptr == libc::MAP_FAILED {
            unsafe { libc::close(fd) };
            return Err(IpcError::Io(std::io::Error::last_os_error()));
        }

        Ok(SharedMemory {
            name: shm_name,
            ptr: NonNull::new(ptr as *mut u8).unwrap(),
            size,
            is_owner: false,
            fd,
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
    use windows_sys::Win32::System::Memory::*;

    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(Some(0)).collect()
    }

    pub fn create_shm(name: &str, size: usize) -> Result<SharedMemory> {
        let wide_name = to_wide(name);

        let handle = unsafe {
            CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                ptr::null(),
                PAGE_READWRITE,
                (size >> 32) as u32,
                size as u32,
                wide_name.as_ptr(),
            )
        };

        if handle.is_null() {
            return Err(IpcError::Io(std::io::Error::last_os_error()));
        }

        // Check if it already existed
        let last_error = unsafe { GetLastError() };
        if last_error == ERROR_ALREADY_EXISTS {
            unsafe { CloseHandle(handle) };
            return Err(IpcError::AlreadyExists(name.to_string()));
        }

        let mapped = unsafe { MapViewOfFile(handle, FILE_MAP_ALL_ACCESS, 0, 0, size) };

        if mapped.Value.is_null() {
            unsafe { CloseHandle(handle) };
            return Err(IpcError::Io(std::io::Error::last_os_error()));
        }

        Ok(SharedMemory {
            name: name.to_string(),
            ptr: NonNull::new(mapped.Value as *mut u8).unwrap(),
            size,
            is_owner: true,
            handle,
        })
    }

    pub fn open_shm(name: &str) -> Result<SharedMemory> {
        let wide_name = to_wide(name);

        let handle = unsafe { OpenFileMappingW(FILE_MAP_ALL_ACCESS, 0, wide_name.as_ptr()) };

        if handle.is_null() {
            let err = std::io::Error::last_os_error();
            return Err(match err.raw_os_error() {
                Some(2) => IpcError::NotFound(name.to_string()),
                Some(5) => IpcError::PermissionDenied(name.to_string()),
                _ => IpcError::Io(err),
            });
        }

        // Map the entire file mapping
        let mapped = unsafe { MapViewOfFile(handle, FILE_MAP_ALL_ACCESS, 0, 0, 0) };

        if mapped.Value.is_null() {
            unsafe { CloseHandle(handle) };
            return Err(IpcError::Io(std::io::Error::last_os_error()));
        }

        // Get the size using VirtualQuery
        let mut info: MEMORY_BASIC_INFORMATION = unsafe { std::mem::zeroed() };
        let ret = unsafe {
            VirtualQuery(
                mapped.Value,
                &mut info,
                std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
            )
        };

        if ret == 0 {
            unsafe {
                UnmapViewOfFile(mapped);
                CloseHandle(handle);
            }
            return Err(IpcError::Io(std::io::Error::last_os_error()));
        }

        Ok(SharedMemory {
            name: name.to_string(),
            ptr: NonNull::new(mapped.Value as *mut u8).unwrap(),
            size: info.RegionSize,
            is_owner: false,
            handle,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared_memory_create_and_write() {
        let name = format!("test_shm_{}", std::process::id());
        let mut shm = SharedMemory::create(&name, 1024).unwrap();

        let data = b"Hello, shared memory!";
        shm.write(0, data).unwrap();

        let read_data = shm.read(0, data.len()).unwrap();
        assert_eq!(read_data, data);
    }

    #[test]
    fn test_shared_memory_boundary() {
        let name = format!("test_shm_boundary_{}", std::process::id());
        let mut shm = SharedMemory::create(&name, 100).unwrap();

        // Should fail - writing beyond boundary
        let result = shm.write(90, &[0u8; 20]);
        assert!(result.is_err());
    }
}
