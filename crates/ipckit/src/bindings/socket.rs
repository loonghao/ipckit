//! Python bindings for LocalSocket types
//!
//! This module provides Python bindings for LocalSocketListener and LocalSocketStream.

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use std::io::{Read, Write};

use super::json_utils::{json_value_to_py, py_to_json_value};
use crate::error::IpcError;
use crate::local_socket::{
    LocalSocketListener as RustLocalSocketListener, LocalSocketStream as RustLocalSocketStream,
};

/// Python wrapper for LocalSocketListener - Server-side local socket
///
/// This class provides a cross-platform local socket server:
/// - On Unix: Uses Unix Domain Sockets
/// - On Windows: Uses Named Pipes
///
/// When the `backend-interprocess` feature is enabled, uses the more robust
/// interprocess crate implementation.
#[pyclass(name = "LocalSocketListener")]
pub struct PyLocalSocketListener {
    inner: parking_lot::Mutex<RustLocalSocketListener>,
}

#[pymethods]
impl PyLocalSocketListener {
    /// Create a new local socket listener bound to the given name
    ///
    /// Args:
    ///     name: The socket name. On Unix, this will be a path like /tmp/name.sock.
    ///           On Windows, this will be a named pipe like \\.\pipe\name.
    #[new]
    fn new(name: &str) -> PyResult<Self> {
        let inner = RustLocalSocketListener::bind(name)?;
        Ok(Self {
            inner: parking_lot::Mutex::new(inner),
        })
    }

    /// Bind to a local socket (alias for __new__)
    #[staticmethod]
    fn bind(name: &str) -> PyResult<Self> {
        Self::new(name)
    }

    /// Accept a new incoming connection
    ///
    /// This method blocks until a client connects.
    /// Returns a LocalSocketStream for bidirectional communication.
    fn accept(&self, _py: Python<'_>) -> PyResult<PyLocalSocketStream> {
        let guard = self.inner.lock();
        let stream = guard.accept()?;
        Ok(PyLocalSocketStream {
            inner: parking_lot::Mutex::new(stream),
        })
    }

    /// Get the name of this listener
    #[getter]
    fn name(&self) -> String {
        self.inner.lock().name().to_string()
    }
}

/// Python wrapper for LocalSocketStream - Bidirectional local socket connection
///
/// This class provides a cross-platform local socket stream:
/// - On Unix: Uses Unix Domain Sockets
/// - On Windows: Uses Named Pipes
///
/// Can be created by:
/// - Calling LocalSocketListener.accept() on the server side
/// - Calling LocalSocketStream.connect() on the client side
#[pyclass(name = "LocalSocketStream")]
pub struct PyLocalSocketStream {
    inner: parking_lot::Mutex<RustLocalSocketStream>,
}

#[pymethods]
impl PyLocalSocketStream {
    /// Connect to a local socket server
    ///
    /// Args:
    ///     name: The socket name to connect to.
    #[staticmethod]
    fn connect(name: &str) -> PyResult<Self> {
        let inner = RustLocalSocketStream::connect(name)?;
        Ok(Self {
            inner: parking_lot::Mutex::new(inner),
        })
    }

    /// Get the name of this stream
    #[getter]
    fn name(&self) -> String {
        self.inner.lock().name().to_string()
    }

    /// Read data from the socket
    ///
    /// Args:
    ///     size: Maximum number of bytes to read
    ///
    /// Returns:
    ///     bytes: The data read from the socket
    fn read(&self, py: Python<'_>, size: usize) -> PyResult<Py<PyBytes>> {
        let mut buf = vec![0u8; size];
        let n = {
            let mut guard = self.inner.lock();
            guard.read(&mut buf)?
        };
        buf.truncate(n);
        Ok(PyBytes::new(py, &buf).into())
    }

    /// Write data to the socket
    ///
    /// Args:
    ///     data: The data to write
    ///
    /// Returns:
    ///     int: Number of bytes written
    fn write(&self, _py: Python<'_>, data: Vec<u8>) -> PyResult<usize> {
        let mut guard = self.inner.lock();
        let n = guard.write(&data)?;
        Ok(n)
    }

    /// Read exact number of bytes
    ///
    /// Args:
    ///     size: Exact number of bytes to read
    ///
    /// Returns:
    ///     bytes: The data read from the socket
    fn read_exact(&self, py: Python<'_>, size: usize) -> PyResult<Py<PyBytes>> {
        let mut buf = vec![0u8; size];
        {
            let mut guard = self.inner.lock();
            guard.read_exact(&mut buf)?;
        }
        Ok(PyBytes::new(py, &buf).into())
    }

    /// Write all data
    ///
    /// Args:
    ///     data: The data to write (all bytes will be written)
    fn write_all(&self, _py: Python<'_>, data: Vec<u8>) -> PyResult<()> {
        let mut guard = self.inner.lock();
        guard.write_all(&data)?;
        Ok(())
    }

    /// Flush the socket
    fn flush(&self, _py: Python<'_>) -> PyResult<()> {
        let mut guard = self.inner.lock();
        guard.flush()?;
        Ok(())
    }

    /// Send a JSON-serializable object
    fn send_json(&self, _py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<()> {
        let value = py_to_json_value(obj)?;
        let json_bytes = serde_json::to_vec(&value)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

        // Send length prefix (4 bytes, big-endian)
        let len_bytes = (json_bytes.len() as u32).to_be_bytes();

        let mut guard = self.inner.lock();
        guard.write_all(&len_bytes)?;
        guard.write_all(&json_bytes)?;
        guard.flush()?;

        Ok(())
    }

    /// Receive a JSON object
    fn recv_json(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let mut guard = self.inner.lock();

        // Read length prefix (4 bytes, big-endian)
        let mut len_bytes = [0u8; 4];
        guard.read_exact(&mut len_bytes)?;
        let len = u32::from_be_bytes(len_bytes) as usize;

        // Read JSON data
        let mut json_bytes = vec![0u8; len];
        guard.read_exact(&mut json_bytes)?;
        drop(guard);

        let value: serde_json::Value = serde_json::from_slice(&json_bytes)
            .map_err(|e| IpcError::deserialization(e.to_string()))?;
        json_value_to_py(py, &value)
    }
}
