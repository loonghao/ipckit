//! Python bindings for pipe types
//!
//! This module provides Python bindings for AnonymousPipe and NamedPipe.

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use std::io::{Read, Write};

use crate::error::IpcError;
use crate::pipe::{AnonymousPipe as RustAnonymousPipe, NamedPipe as RustNamedPipe};

/// Python wrapper for AnonymousPipe
/// Uses Mutex to allow concurrent access from multiple threads
#[pyclass(name = "AnonymousPipe")]
pub struct PyAnonymousPipe {
    reader: std::sync::Mutex<Option<crate::pipe::PipeReader>>,
    writer: std::sync::Mutex<Option<crate::pipe::PipeWriter>>,
}

#[pymethods]
impl PyAnonymousPipe {
    /// Create a new anonymous pipe pair
    #[new]
    fn new() -> PyResult<Self> {
        let pipe = RustAnonymousPipe::new()?;
        let (reader, writer) = pipe.split();
        Ok(Self {
            reader: std::sync::Mutex::new(Some(reader)),
            writer: std::sync::Mutex::new(Some(writer)),
        })
    }

    /// Read data from the pipe
    fn read(&self, py: Python<'_>, size: usize) -> PyResult<Py<PyBytes>> {
        let mut guard = self
            .reader
            .lock()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Lock poisoned"))?;
        let reader = guard.as_mut().ok_or(IpcError::Closed)?;

        let mut buf = vec![0u8; size];
        let n = py.allow_threads(|| reader.read(&mut buf))?;
        buf.truncate(n);

        Ok(PyBytes::new(py, &buf).into())
    }

    /// Write data to the pipe
    fn write(&self, py: Python<'_>, data: &[u8]) -> PyResult<usize> {
        let mut guard = self
            .writer
            .lock()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Lock poisoned"))?;
        let writer = guard.as_mut().ok_or(IpcError::Closed)?;
        let data = data.to_vec(); // Clone data before releasing GIL
        let n = py.allow_threads(|| writer.write(&data))?;
        Ok(n)
    }

    /// Get the reader file descriptor (Unix only)
    #[cfg(unix)]
    fn reader_fd(&self) -> PyResult<i32> {
        use std::os::unix::io::AsRawFd;
        let guard = self
            .reader
            .lock()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Lock poisoned"))?;
        let reader = guard.as_ref().ok_or(IpcError::Closed)?;
        Ok(reader.as_raw_fd())
    }

    /// Get the writer file descriptor (Unix only)
    #[cfg(unix)]
    fn writer_fd(&self) -> PyResult<i32> {
        use std::os::unix::io::AsRawFd;
        let guard = self
            .writer
            .lock()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Lock poisoned"))?;
        let writer = guard.as_ref().ok_or(IpcError::Closed)?;
        Ok(writer.as_raw_fd())
    }

    /// Take the reader end (for passing to child process)
    fn take_reader(&self) -> PyResult<()> {
        let mut guard = self
            .reader
            .lock()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Lock poisoned"))?;
        guard.take();
        Ok(())
    }

    /// Take the writer end (for passing to child process)
    fn take_writer(&self) -> PyResult<()> {
        let mut guard = self
            .writer
            .lock()
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Lock poisoned"))?;
        guard.take();
        Ok(())
    }
}

/// Python wrapper for NamedPipe
#[pyclass(name = "NamedPipe")]
pub struct PyNamedPipe {
    inner: RustNamedPipe,
}

#[pymethods]
impl PyNamedPipe {
    /// Create a new named pipe server
    #[staticmethod]
    fn create(name: &str) -> PyResult<Self> {
        let inner = RustNamedPipe::create(name)?;
        Ok(Self { inner })
    }

    /// Connect to an existing named pipe
    #[staticmethod]
    fn connect(name: &str) -> PyResult<Self> {
        let inner = RustNamedPipe::connect(name)?;
        Ok(Self { inner })
    }

    /// Get the pipe name
    #[getter]
    fn name(&self) -> &str {
        self.inner.name()
    }

    /// Check if this is the server end
    #[getter]
    fn is_server(&self) -> bool {
        self.inner.is_server()
    }

    /// Wait for a client to connect (server only)
    fn wait_for_client(&mut self, py: Python<'_>) -> PyResult<()> {
        // Release GIL to allow other Python threads to run
        py.allow_threads(|| self.inner.wait_for_client())?;
        Ok(())
    }

    /// Read data from the pipe
    fn read(&mut self, py: Python<'_>, size: usize) -> PyResult<Py<PyBytes>> {
        let mut buf = vec![0u8; size];
        // Release GIL during blocking read
        let n = py.allow_threads(|| self.inner.read(&mut buf))?;
        buf.truncate(n);
        Ok(PyBytes::new(py, &buf).into())
    }

    /// Write data to the pipe
    fn write(&mut self, py: Python<'_>, data: Vec<u8>) -> PyResult<usize> {
        // Release GIL during write
        let n = py.allow_threads(|| self.inner.write(&data))?;
        Ok(n)
    }

    /// Read exact number of bytes
    fn read_exact(&mut self, py: Python<'_>, size: usize) -> PyResult<Py<PyBytes>> {
        let mut buf = vec![0u8; size];
        // Release GIL during blocking read
        py.allow_threads(|| self.inner.read_exact(&mut buf))?;
        Ok(PyBytes::new(py, &buf).into())
    }

    /// Write all data
    fn write_all(&mut self, py: Python<'_>, data: Vec<u8>) -> PyResult<()> {
        // Release GIL during write
        py.allow_threads(|| self.inner.write_all(&data))?;
        Ok(())
    }
}
