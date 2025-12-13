//! Python bindings for ipckit
//!
//! This module provides Python bindings using PyO3.
//! All JSON serialization is done in Rust using serde_json for better performance.
//!
//! The bindings are organized into submodules:
//! - `json_utils`: JSON conversion utilities (py_to_json_value, json_value_to_py)
//! - `pipe`: AnonymousPipe and NamedPipe bindings
//! - `shm`: SharedMemory bindings
//! - `channel`: IpcChannel and FileChannel bindings
//! - `graceful`: GracefulNamedPipe and GracefulIpcChannel bindings
//! - `socket`: LocalSocketListener and LocalSocketStream bindings

mod channel;
mod graceful;
mod json_utils;
mod pipe;
mod shm;
mod socket;

// Re-export all Python classes
pub use channel::{PyFileChannel, PyIpcChannel};
pub use graceful::{PyGracefulIpcChannel, PyGracefulNamedPipe};
pub use json_utils::{
    json_dumps, json_dumps_pretty, json_loads, json_value_to_py, py_to_json_value,
};
pub use pipe::{PyAnonymousPipe, PyNamedPipe};
pub use shm::PySharedMemory;
pub use socket::{PyLocalSocketListener, PyLocalSocketStream};

use pyo3::prelude::*;

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

    // Local socket classes (Issue #18: Socket Server)
    m.add_class::<PyLocalSocketListener>()?;
    m.add_class::<PyLocalSocketStream>()?;

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
- LocalSocketListener: Server-side local socket (Unix Domain Socket / Named Pipe)
- LocalSocketStream: Client-side local socket connection

Graceful shutdown support:
- GracefulNamedPipe: Named pipe with graceful shutdown
- GracefulIpcChannel: IPC channel with graceful shutdown

JSON utilities (faster than Python's json module):
- json_dumps(obj): Serialize Python object to JSON string
- json_dumps_pretty(obj): Serialize with pretty formatting
- json_loads(s): Deserialize JSON string to Python object

Example:
    import ipckit

    # Server side
    listener = ipckit.LocalSocketListener.bind('my_socket')
    stream = listener.accept()
    data = stream.read(1024)
    stream.write(b'Hello from server!')

    # Client side
    stream = ipckit.LocalSocketStream.connect('my_socket')
    stream.write(b'Hello from client!')
    response = stream.read(1024)

    # Using JSON messaging
    stream.send_json({'type': 'request', 'data': [1, 2, 3]})
    response = stream.recv_json()
",
    )?;

    Ok(())
}
