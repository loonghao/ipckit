//! Python bindings for IpcChannel and FileChannel
//!
//! This module provides Python bindings for channel-based IPC.

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};
use std::time::Duration;

use super::json_utils::{json_value_to_py, py_to_json_value};
use crate::error::IpcError;
use crate::file_channel::{
    FileChannel as RustFileChannel, FileMessage as RustFileMessage, MessageType as RustMessageType,
};

/// Python wrapper for IpcChannel
#[pyclass(name = "IpcChannel")]
pub struct PyIpcChannel {
    inner: crate::channel::IpcChannel<Vec<u8>>,
}

#[pymethods]
impl PyIpcChannel {
    /// Create a new IPC channel server
    #[staticmethod]
    fn create(name: &str) -> PyResult<Self> {
        let inner = crate::channel::IpcChannel::create(name)?;
        Ok(Self { inner })
    }

    /// Connect to an existing IPC channel
    #[staticmethod]
    fn connect(name: &str) -> PyResult<Self> {
        let inner = crate::channel::IpcChannel::connect(name)?;
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

    /// Wait for a client to connect (server only)
    fn wait_for_client(&mut self, py: Python<'_>) -> PyResult<()> {
        // Release GIL to allow other Python threads to run
        py.allow_threads(|| self.inner.wait_for_client())?;
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

/// Python wrapper for FileChannel - File-based IPC for frontend-backend communication
///
/// All JSON serialization is done in Rust for better performance.
#[pyclass(name = "FileChannel")]
pub struct PyFileChannel {
    inner: RustFileChannel,
}

#[pymethods]
impl PyFileChannel {
    /// Create a backend-side file channel
    #[staticmethod]
    fn backend(dir: &str) -> PyResult<Self> {
        let inner = RustFileChannel::backend(dir)?;
        Ok(Self { inner })
    }

    /// Create a frontend-side file channel
    #[staticmethod]
    fn frontend(dir: &str) -> PyResult<Self> {
        let inner = RustFileChannel::frontend(dir)?;
        Ok(Self { inner })
    }

    /// Get the channel directory path
    #[getter]
    fn dir(&self) -> String {
        self.inner.dir().to_string_lossy().to_string()
    }

    /// Send a request message (JSON serialization done in Rust)
    fn send_request(&self, method: &str, params: &Bound<'_, PyAny>) -> PyResult<String> {
        let json_value = py_to_json_value(params)?;
        let id = self.inner.send_request(method, json_value)?;
        Ok(id)
    }

    /// Send a response to a request
    fn send_response(&self, request_id: &str, result: &Bound<'_, PyAny>) -> PyResult<()> {
        let json_value = py_to_json_value(result)?;
        self.inner.send_response(request_id, json_value)?;
        Ok(())
    }

    /// Send an error response
    fn send_error(&self, request_id: &str, error: &str) -> PyResult<()> {
        self.inner.send_error(request_id, error)?;
        Ok(())
    }

    /// Send an event (fire-and-forget, no response expected)
    fn send_event(&self, name: &str, payload: &Bound<'_, PyAny>) -> PyResult<()> {
        let json_value = py_to_json_value(payload)?;
        self.inner.send_event(name, json_value)?;
        Ok(())
    }

    /// Receive all new messages
    fn recv(&mut self, py: Python<'_>) -> PyResult<PyObject> {
        let messages = self.inner.recv()?;
        let list = PyList::empty(py);
        for msg in messages {
            list.append(file_message_to_py(py, msg)?)?;
        }
        Ok(list.into())
    }

    /// Receive a single new message (non-blocking)
    fn recv_one(&mut self, py: Python<'_>) -> PyResult<PyObject> {
        match self.inner.recv_one()? {
            Some(msg) => file_message_to_py(py, msg),
            None => Ok(py.None()),
        }
    }

    /// Wait for a response to a specific request
    fn wait_response(
        &mut self,
        py: Python<'_>,
        request_id: &str,
        timeout_ms: u64,
    ) -> PyResult<PyObject> {
        let timeout = Duration::from_millis(timeout_ms);
        let msg = self.inner.wait_response(request_id, timeout)?;
        file_message_to_py(py, msg)
    }

    /// Clear all messages in both inbox and outbox
    fn clear(&self) -> PyResult<()> {
        self.inner.clear()?;
        Ok(())
    }
}

/// Convert FileMessage to Python dict (all in Rust, no Python json module)
fn file_message_to_py(py: Python<'_>, msg: RustFileMessage) -> PyResult<PyObject> {
    let dict = PyDict::new(py);

    dict.set_item("id", &msg.id)?;
    dict.set_item("timestamp", msg.timestamp)?;

    let msg_type = match msg.msg_type {
        RustMessageType::Request => "request",
        RustMessageType::Response => "response",
        RustMessageType::Event => "event",
    };
    dict.set_item("type", msg_type)?;

    if let Some(method) = msg.method {
        dict.set_item("method", method)?;
    }

    if let Some(reply_to) = msg.reply_to {
        dict.set_item("reply_to", reply_to)?;
    }

    if let Some(error) = msg.error {
        dict.set_item("error", error)?;
    }

    // Convert payload using Rust JSON conversion
    let payload = json_value_to_py(py, &msg.payload)?;
    dict.set_item("payload", payload)?;

    Ok(dict.into())
}
