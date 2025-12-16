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
//! - `cli_bridge`: CLI Bridge bindings for CLI tool integration
//! - `metrics`: ChannelMetrics bindings for performance monitoring
//! - `api_server`: API Server bindings for HTTP-over-Socket RESTful API

mod api_server;
mod channel;
mod cli_bridge;
mod graceful;
mod json_utils;
mod metrics;
mod pipe;
mod shm;
mod socket;

// Re-export all Python classes
pub use api_server::{PyApiClient, PyApiServerConfig, PyRequest, PyResponse};
pub use channel::{PyFileChannel, PyIpcChannel};
pub use cli_bridge::{
    parse_progress, wrap_command, PyCliBridge, PyCliBridgeConfig, PyCommandOutput, PyProgressInfo,
};
pub use graceful::{PyGracefulIpcChannel, PyGracefulNamedPipe};
pub use json_utils::{
    json_dumps, json_dumps_pretty, json_loads, json_value_to_py, py_to_json_value,
};
pub use metrics::{PyChannelMetrics, PyMetricsSnapshot};
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

    // CLI Bridge classes (Issue #17: CLI Bridge)
    m.add_class::<PyCliBridge>()?;
    m.add_class::<PyCliBridgeConfig>()?;
    m.add_class::<PyProgressInfo>()?;
    m.add_class::<PyCommandOutput>()?;
    m.add_function(wrap_pyfunction!(wrap_command, m)?)?;
    m.add_function(wrap_pyfunction!(parse_progress, m)?)?;

    // Metrics classes (Issue #10: ChannelMetrics)
    m.add_class::<PyChannelMetrics>()?;
    m.add_class::<PyMetricsSnapshot>()?;

    // API Server classes (Issue #14: API Server)
    m.add_class::<PyApiServerConfig>()?;
    m.add_class::<PyRequest>()?;
    m.add_class::<PyResponse>()?;
    m.add_class::<PyApiClient>()?;

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

CLI Bridge (for CLI tool integration):
- CliBridge: Bridge for CLI tools to communicate with frontends
- CliBridgeConfig: Configuration for CLI bridge
- wrap_command(): Wrap a subprocess with CLI bridge integration
- parse_progress(): Parse progress from output lines

Metrics (Issue #10: Performance monitoring):
- ChannelMetrics: Track message counts, latency, throughput
- MetricsSnapshot: Point-in-time snapshot of metrics

API Server (Issue #14: HTTP-over-Socket RESTful API):
- ApiServerConfig: Configuration for API server
- Request: HTTP request object
- Response: HTTP response object
- ApiClient: Client for making API requests

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

    # CLI Bridge usage
    bridge = ipckit.CliBridge.connect()
    bridge.register_task('My Task', 'custom')
    bridge.set_progress(50, 'Half done')
    bridge.complete({'success': True})

    # Wrap a subprocess
    output = ipckit.wrap_command(['pip', 'install', 'requests'], task_name='Install')
    print(f'Exit code: {output.exit_code}')

    # Metrics usage
    metrics = ipckit.ChannelMetrics()
    metrics.record_send(100)
    print(f'Messages sent: {metrics.messages_sent}')
    print(metrics.to_prometheus('ipckit'))

    # API Client usage
    client = ipckit.ApiClient.connect()
    tasks = client.get('/v1/tasks')
",
    )?;

    Ok(())
}
