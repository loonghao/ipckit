//! Python bindings for ipckit
//!
//! This module provides Python bindings using PyO3.
//! All JSON serialization is done in Rust using serde_json for better performance.

use pyo3::prelude::*;
use pyo3::types::{PyBool, PyBytes, PyDict, PyFloat, PyInt, PyList, PyString};
use std::io::{Read, Write};
use std::time::Duration;

use crate::error::IpcError;
use crate::file_channel::{
    FileChannel as RustFileChannel, FileMessage as RustFileMessage, MessageType as RustMessageType,
};
use crate::graceful::{
    GracefulChannel, GracefulIpcChannel as RustGracefulIpcChannel,
    GracefulNamedPipe as RustGracefulNamedPipe,
};
use crate::pipe::{AnonymousPipe as RustAnonymousPipe, NamedPipe as RustNamedPipe};
use crate::shm::SharedMemory as RustSharedMemory;

// ============================================================================
// JSON Conversion Utilities (Rust-native, no Python json module dependency)
// ============================================================================

/// Convert a Python object to serde_json::Value using Rust
/// This is faster than using Python's json module
fn py_to_json_value(obj: &Bound<'_, PyAny>) -> PyResult<serde_json::Value> {
    // None
    if obj.is_none() {
        return Ok(serde_json::Value::Null);
    }

    // Bool (must check before int, as bool is subclass of int in Python)
    if let Ok(b) = obj.downcast::<PyBool>() {
        return Ok(serde_json::Value::Bool(b.is_true()));
    }

    // Int
    if let Ok(i) = obj.downcast::<PyInt>() {
        if let Ok(v) = i.extract::<i64>() {
            return Ok(serde_json::Value::Number(v.into()));
        }
        if let Ok(v) = i.extract::<u64>() {
            return Ok(serde_json::Value::Number(v.into()));
        }
        // Fall back to float for very large integers
        if let Ok(v) = i.extract::<f64>() {
            if let Some(n) = serde_json::Number::from_f64(v) {
                return Ok(serde_json::Value::Number(n));
            }
        }
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Integer too large for JSON",
        ));
    }

    // Float
    if let Ok(f) = obj.downcast::<PyFloat>() {
        let v: f64 = f.extract()?;
        if let Some(n) = serde_json::Number::from_f64(v) {
            return Ok(serde_json::Value::Number(n));
        }
        // NaN and Infinity are not valid JSON
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "Float value is not valid JSON (NaN or Infinity)",
        ));
    }

    // String
    if let Ok(s) = obj.downcast::<PyString>() {
        let v: String = s.extract()?;
        return Ok(serde_json::Value::String(v));
    }

    // Bytes -> base64 string
    if let Ok(b) = obj.downcast::<PyBytes>() {
        let bytes: &[u8] = b.as_bytes();
        // Use base64 encoding for bytes
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        return Ok(serde_json::Value::String(encoded));
    }

    // List
    if let Ok(list) = obj.downcast::<PyList>() {
        let mut arr = Vec::with_capacity(list.len());
        for item in list.iter() {
            arr.push(py_to_json_value(&item)?);
        }
        return Ok(serde_json::Value::Array(arr));
    }

    // Dict
    if let Ok(dict) = obj.downcast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (key, value) in dict.iter() {
            let key_str: String = key.extract().map_err(|_| {
                PyErr::new::<pyo3::exceptions::PyTypeError, _>("Dict keys must be strings")
            })?;
            map.insert(key_str, py_to_json_value(&value)?);
        }
        return Ok(serde_json::Value::Object(map));
    }

    // Try to convert other types via their __dict__ or repr
    Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
        "Cannot convert type to JSON: {:?}",
        obj.get_type().name()
    )))
}

/// Convert serde_json::Value to Python object
fn json_value_to_py(py: Python<'_>, value: &serde_json::Value) -> PyResult<PyObject> {
    match value {
        serde_json::Value::Null => Ok(py.None()),
        serde_json::Value::Bool(b) => Ok(b.into_pyobject(py)?.to_owned().into_any().unbind()),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_pyobject(py)?.into_any().unbind())
            } else if let Some(u) = n.as_u64() {
                Ok(u.into_pyobject(py)?.into_any().unbind())
            } else if let Some(f) = n.as_f64() {
                Ok(f.into_pyobject(py)?.into_any().unbind())
            } else {
                Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "Invalid JSON number",
                ))
            }
        }
        serde_json::Value::String(s) => Ok(s.into_pyobject(py)?.into_any().unbind()),
        serde_json::Value::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                list.append(json_value_to_py(py, item)?)?;
            }
            Ok(list.into())
        }
        serde_json::Value::Object(map) => {
            let dict = PyDict::new(py);
            for (key, value) in map {
                dict.set_item(key, json_value_to_py(py, value)?)?;
            }
            Ok(dict.into())
        }
    }
}

/// Serialize Python object to JSON bytes using Rust's serde_json
#[pyfunction]
fn json_dumps(obj: &Bound<'_, PyAny>) -> PyResult<String> {
    let value = py_to_json_value(obj)?;
    serde_json::to_string(&value)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
}

/// Serialize Python object to pretty JSON string
#[pyfunction]
fn json_dumps_pretty(obj: &Bound<'_, PyAny>) -> PyResult<String> {
    let value = py_to_json_value(obj)?;
    serde_json::to_string_pretty(&value)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
}

/// Deserialize JSON string to Python object using Rust's serde_json
#[pyfunction]
fn json_loads(py: Python<'_>, s: &str) -> PyResult<PyObject> {
    let value: serde_json::Value = serde_json::from_str(s)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    json_value_to_py(py, &value)
}

// ============================================================================
// AnonymousPipe
// ============================================================================

/// Python wrapper for AnonymousPipe
#[pyclass(name = "AnonymousPipe")]
pub struct PyAnonymousPipe {
    reader: Option<crate::pipe::PipeReader>,
    writer: Option<crate::pipe::PipeWriter>,
}

#[pymethods]
impl PyAnonymousPipe {
    /// Create a new anonymous pipe pair
    #[new]
    fn new() -> PyResult<Self> {
        let pipe = RustAnonymousPipe::new()?;
        let (reader, writer) = pipe.split();
        Ok(Self {
            reader: Some(reader),
            writer: Some(writer),
        })
    }

    /// Read data from the pipe
    fn read(&mut self, py: Python<'_>, size: usize) -> PyResult<Py<PyBytes>> {
        let reader = self.reader.as_mut().ok_or_else(|| IpcError::Closed)?;

        let mut buf = vec![0u8; size];
        let n = reader.read(&mut buf)?;
        buf.truncate(n);

        Ok(PyBytes::new(py, &buf).into())
    }

    /// Write data to the pipe
    fn write(&mut self, data: &[u8]) -> PyResult<usize> {
        let writer = self.writer.as_mut().ok_or_else(|| IpcError::Closed)?;
        let n = writer.write(data)?;
        Ok(n)
    }

    /// Get the reader file descriptor (Unix only)
    #[cfg(unix)]
    fn reader_fd(&self) -> PyResult<i32> {
        use std::os::unix::io::AsRawFd;
        let reader = self.reader.as_ref().ok_or_else(|| IpcError::Closed)?;
        Ok(reader.as_raw_fd())
    }

    /// Get the writer file descriptor (Unix only)
    #[cfg(unix)]
    fn writer_fd(&self) -> PyResult<i32> {
        use std::os::unix::io::AsRawFd;
        let writer = self.writer.as_ref().ok_or_else(|| IpcError::Closed)?;
        Ok(writer.as_raw_fd())
    }

    /// Take the reader end (for passing to child process)
    fn take_reader(&mut self) -> PyResult<()> {
        self.reader.take();
        Ok(())
    }

    /// Take the writer end (for passing to child process)
    fn take_writer(&mut self) -> PyResult<()> {
        self.writer.take();
        Ok(())
    }
}

// ============================================================================
// NamedPipe
// ============================================================================

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
    fn wait_for_client(&self, py: Python<'_>) -> PyResult<()> {
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

// ============================================================================
// SharedMemory
// ============================================================================

/// Python wrapper for SharedMemory
#[pyclass(name = "SharedMemory")]
pub struct PySharedMemory {
    inner: RustSharedMemory,
}

#[pymethods]
impl PySharedMemory {
    /// Create a new shared memory region
    #[staticmethod]
    fn create(name: &str, size: usize) -> PyResult<Self> {
        let inner = RustSharedMemory::create(name, size)?;
        Ok(Self { inner })
    }

    /// Open an existing shared memory region
    #[staticmethod]
    fn open(name: &str) -> PyResult<Self> {
        let inner = RustSharedMemory::open(name)?;
        Ok(Self { inner })
    }

    /// Get the shared memory name
    #[getter]
    fn name(&self) -> &str {
        self.inner.name()
    }

    /// Get the shared memory size
    #[getter]
    fn size(&self) -> usize {
        self.inner.size()
    }

    /// Check if this instance is the owner
    #[getter]
    fn is_owner(&self) -> bool {
        self.inner.is_owner()
    }

    /// Write data to shared memory at offset
    fn write(&mut self, offset: usize, data: &[u8]) -> PyResult<()> {
        self.inner.write(offset, data)?;
        Ok(())
    }

    /// Read data from shared memory at offset
    fn read(&self, py: Python<'_>, offset: usize, size: usize) -> PyResult<Py<PyBytes>> {
        let data = self.inner.read(offset, size)?;
        Ok(PyBytes::new(py, &data).into())
    }

    /// Read all data from shared memory
    fn read_all(&self, py: Python<'_>) -> PyResult<Py<PyBytes>> {
        let data = self.inner.read(0, self.inner.size())?;
        Ok(PyBytes::new(py, &data).into())
    }
}

// ============================================================================
// IpcChannel
// ============================================================================

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
    fn wait_for_client(&self, py: Python<'_>) -> PyResult<()> {
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
        let value: serde_json::Value = serde_json::from_slice(&data)
            .map_err(|e| IpcError::deserialization(e.to_string()))?;
        json_value_to_py(py, &value)
    }
}

// ============================================================================
// FileChannel
// ============================================================================

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

// ============================================================================
// GracefulNamedPipe
// ============================================================================

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
    fn wait_for_client(&self, py: Python<'_>) -> PyResult<()> {
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

// ============================================================================
// GracefulIpcChannel
// ============================================================================

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
    fn wait_for_client(&self, py: Python<'_>) -> PyResult<()> {
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
        let value: serde_json::Value = serde_json::from_slice(&data)
            .map_err(|e| IpcError::deserialization(e.to_string()))?;
        json_value_to_py(py, &value)
    }
}

// ============================================================================
// Python Module
// ============================================================================

/// Create the Python module
#[pymodule]
#[pyo3(name = "ipckit")]
pub fn ipckit_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // IPC classes
    m.add_class::<PyAnonymousPipe>()?;
    m.add_class::<PyNamedPipe>()?;
    m.add_class::<PySharedMemory>()?;
    m.add_class::<PyIpcChannel>()?;
    m.add_class::<PyFileChannel>()?;
    
    // Graceful shutdown classes
    m.add_class::<PyGracefulNamedPipe>()?;
    m.add_class::<PyGracefulIpcChannel>()?;

    // JSON utilities (Rust-native, faster than Python's json module)
    m.add_function(wrap_pyfunction!(json_dumps, m)?)?;
    m.add_function(wrap_pyfunction!(json_dumps_pretty, m)?)?;
    m.add_function(wrap_pyfunction!(json_loads, m)?)?;

    // Version info
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    // Docstring
    m.add(
        "__doc__",
        "ipckit - A cross-platform IPC library for Python

This library provides various IPC mechanisms:
- AnonymousPipe: For parent-child process communication
- NamedPipe: For communication between unrelated processes
- SharedMemory: For fast data sharing between processes
- IpcChannel: High-level message passing interface
- FileChannel: File-based IPC for frontend-backend communication

Graceful shutdown support:
- GracefulNamedPipe: Named pipe with graceful shutdown
- GracefulIpcChannel: IPC channel with graceful shutdown

JSON utilities (faster than Python's json module):
- json_dumps(obj): Serialize Python object to JSON string
- json_dumps_pretty(obj): Serialize with pretty formatting
- json_loads(s): Deserialize JSON string to Python object

Example:
    import ipckit
    
    # Using graceful shutdown
    channel = ipckit.GracefulIpcChannel.create('my_channel')
    channel.wait_for_client()
    
    # ... use channel ...
    
    # Graceful shutdown
    channel.shutdown()
    channel.drain()  # Wait for pending operations
    
    # Or with timeout (in milliseconds)
    channel.shutdown_timeout(5000)
",
    )?;

    Ok(())
}
