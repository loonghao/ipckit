//! Python bindings for graceful shutdown channels
//!
//! This module provides Python bindings for GracefulNamedPipe and GracefulIpcChannel.

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use std::io::{Read, Write};
use std::time::Duration;

use super::json_utils::{json_value_to_py, py_to_json_value};
use crate::error::IpcError;
use crate::graceful::{
    GracefulChannel, GracefulIpcChannel as RustGracefulIpcChannel,
    GracefulNamedPipe as RustGracefulNamedPipe,
};

/// Python wrapper for GracefulNamedPipe - Named pipe with graceful shutdown support
///
/// This class wraps a NamedPipe with graceful shutdown capabilities,
/// preventing errors when background threads continue sending messages
/// after the main event loop has closed.
#[pyclass(name = "GracefulNamedPipe")]
pub struct PyGracefulNamedPipe {
    inner: RustGracefulNamedPipe,
}

#[pymethods]
impl PyGracefulNamedPipe {
    /// Create a new named pipe server with graceful shutdown
    #[staticmethod]
    fn create(name: &str) -> PyResult<Self> {
        let inner = RustGracefulNamedPipe::create(name)?;
        Ok(Self { inner })
    }

    /// Connect to an existing named pipe with graceful shutdown
    #[staticmethod]
    fn connect(name: &str) -> PyResult<Self> {
        let inner = RustGracefulNamedPipe::connect(name)?;
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

    /// Check if the channel has been shutdown
    #[getter]
    fn is_shutdown(&self) -> bool {
        self.inner.is_shutdown()
    }

    /// Wait for a client to connect (server only)
    fn wait_for_client(&mut self, py: Python<'_>) -> PyResult<()> {
        // Release GIL to allow other Python threads to run
        py.allow_threads(|| self.inner.wait_for_client())?;
        Ok(())
    }

    /// Signal the channel to shutdown
    ///
    /// After calling this method:
    /// - New send/receive operations will raise ConnectionError
    /// - Pending operations may still complete
    /// - Use drain() to wait for pending operations
    fn shutdown(&self) {
        self.inner.shutdown();
    }

    /// Wait for all pending operations to complete
    fn drain(&self, py: Python<'_>) -> PyResult<()> {
        py.allow_threads(|| self.inner.drain())?;
        Ok(())
    }

    /// Shutdown with a timeout (in milliseconds)
    ///
    /// Combines shutdown() and drain() with a timeout.
    /// Raises TimeoutError if the drain doesn't complete within the timeout.
    fn shutdown_timeout(&self, py: Python<'_>, timeout_ms: u64) -> PyResult<()> {
        let timeout = Duration::from_millis(timeout_ms);
        py.allow_threads(|| self.inner.shutdown_timeout(timeout))?;
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

/// Python wrapper for GracefulIpcChannel - IPC channel with graceful shutdown support
///
/// This class wraps an IpcChannel with graceful shutdown capabilities,
/// preventing errors when background threads continue sending messages
/// after the main event loop has closed.
#[pyclass(name = "GracefulIpcChannel")]
pub struct PyGracefulIpcChannel {
    inner: RustGracefulIpcChannel<Vec<u8>>,
}

#[pymethods]
impl PyGracefulIpcChannel {
    /// Create a new IPC channel server with graceful shutdown
    #[staticmethod]
    fn create(name: &str) -> PyResult<Self> {
        let inner = RustGracefulIpcChannel::create(name)?;
        Ok(Self { inner })
    }

    /// Connect to an existing IPC channel with graceful shutdown
    #[staticmethod]
    fn connect(name: &str) -> PyResult<Self> {
        let inner = RustGracefulIpcChannel::connect(name)?;
        Ok(Self { inner })
    }

    /// Get the channel name
    #[getter]
    fn name(&self) -> &str {
        self.inner.name()
    }

    /// Check if this is the server end
    #[getter]
    fn is_server(&self) -> bool {
        self.inner.is_server()
    }

    /// Check if the channel has been shutdown
    #[getter]
    fn is_shutdown(&self) -> bool {
        self.inner.is_shutdown()
    }

    /// Wait for a client to connect (server only)
    fn wait_for_client(&mut self, py: Python<'_>) -> PyResult<()> {
        // Release GIL to allow other Python threads to run
        py.allow_threads(|| self.inner.wait_for_client())?;
        Ok(())
    }

    /// Signal the channel to shutdown
    ///
    /// After calling this method:
    /// - New send/receive operations will raise ConnectionError
    /// - Pending operations may still complete
    /// - Use drain() to wait for pending operations
    fn shutdown(&self) {
        self.inner.shutdown();
    }

    /// Wait for all pending operations to complete
    fn drain(&self, py: Python<'_>) -> PyResult<()> {
        py.allow_threads(|| self.inner.drain())?;
        Ok(())
    }

    /// Shutdown with a timeout (in milliseconds)
    ///
    /// Combines shutdown() and drain() with a timeout.
    /// Raises TimeoutError if the drain doesn't complete within the timeout.
    fn shutdown_timeout(&self, py: Python<'_>, timeout_ms: u64) -> PyResult<()> {
        let timeout = Duration::from_millis(timeout_ms);
        py.allow_threads(|| self.inner.shutdown_timeout(timeout))?;
        Ok(())
    }

    /// Send bytes through the channel
    fn send(&mut self, py: Python<'_>, data: Vec<u8>) -> PyResult<()> {
        py.allow_threads(|| self.inner.send_bytes(&data))?;
        Ok(())
    }

    /// Receive bytes from the channel
    fn recv(&mut self, py: Python<'_>) -> PyResult<Py<PyBytes>> {
        let data = py.allow_threads(|| self.inner.recv_bytes())?;
        Ok(PyBytes::new(py, &data).into())
    }

    /// Send a JSON-serializable object (uses Rust serde_json)
    fn send_json(&mut self, py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<()> {
        let value = py_to_json_value(obj)?;
        let json_bytes = serde_json::to_vec(&value)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        py.allow_threads(|| self.inner.send_bytes(&json_bytes))?;
        Ok(())
    }

    /// Receive a JSON object (uses Rust serde_json)
    fn recv_json(&mut self, py: Python<'_>) -> PyResult<PyObject> {
        let data = py.allow_threads(|| self.inner.recv_bytes())?;
        let value: serde_json::Value =
            serde_json::from_slice(&data).map_err(|e| IpcError::deserialization(e.to_string()))?;
        json_value_to_py(py, &value)
    }
}
