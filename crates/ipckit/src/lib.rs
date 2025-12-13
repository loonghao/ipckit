//! # ipckit
//!
//! A cross-platform IPC (Inter-Process Communication) library for Rust and Python.
//!
//! ## Features
//!
//! - **Pipes**: Anonymous and named pipes for parent-child process communication
//! - **Shared Memory**: Fast data sharing between processes using memory-mapped regions
//! - **Unix Domain Sockets / Named Pipes**: Bidirectional communication channels
//! - **Message Channels**: High-level message passing with serialization support
//! - **File Channel**: Simple file-based IPC for frontend-backend communication
//! - **Thread Channel**: High-performance intra-process thread communication
//! - **Event Stream**: Real-time publish-subscribe event system
//! - **Task Manager**: Task lifecycle management with progress tracking
//! - **Socket Server**: Multi-client socket server (like Docker's socket)
//!
//! ## Example
//!
//! ```rust,no_run
//! use ipckit::{NamedPipe, IpcError};
//!
//! fn main() -> Result<(), IpcError> {
//!     // Create a named pipe server
//!     let server = NamedPipe::create("my_pipe")?;
//!
//!     // In another process, connect as client
//!     // let client = NamedPipe::connect("my_pipe")?;
//!
//!     Ok(())
//! }
//! ```

pub mod channel;
pub mod error;
pub mod event_stream;
pub mod file_channel;
pub mod graceful;
pub mod local_socket;
pub mod pipe;
pub mod shm;
pub mod socket_server;
pub mod task_manager;
pub mod thread_channel;

#[cfg(unix)]
pub mod unix;

#[cfg(windows)]
pub mod windows;

// Re-exports
pub use channel::{IpcChannel, IpcReceiver, IpcSender};
pub use error::{IpcError, Result};
pub use event_stream::{
    event_types, Event, EventBus, EventBusConfig, EventFilter, EventPublisher, EventSubscriber,
};
pub use file_channel::{FileChannel, FileMessage, MessageType as FileMessageType};
pub use graceful::{
    GracefulChannel, GracefulIpcChannel, GracefulNamedPipe, GracefulWrapper, OperationGuard,
    ShutdownState,
};
pub use local_socket::{LocalSocketListener, LocalSocketStream};
pub use pipe::{AnonymousPipe, NamedPipe, PipeReader, PipeWriter};
pub use shm::SharedMemory;
pub use socket_server::{
    Connection, ConnectionHandler, ConnectionId, ConnectionMetadata, FnHandler, Message,
    SocketClient, SocketServer, SocketServerConfig,
};
pub use task_manager::{
    CancellationToken, TaskBuilder, TaskFilter, TaskHandle, TaskInfo, TaskManager,
    TaskManagerConfig, TaskStatus,
};
pub use thread_channel::{ThreadChannel, ThreadReceiver, ThreadSender};

// Async local socket exports (when both async and backend-interprocess features are enabled)
#[cfg(all(feature = "async", feature = "backend-interprocess"))]
pub use local_socket::{AsyncLocalSocketListener, AsyncLocalSocketStream};

// Python bindings (organized into submodules for better maintainability)
#[cfg(feature = "python-bindings")]
pub mod bindings;

#[cfg(feature = "python-bindings")]
pub use bindings::*;
