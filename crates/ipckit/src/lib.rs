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
pub mod file_channel;
pub mod graceful;
pub mod local_socket;
pub mod pipe;
pub mod shm;

#[cfg(unix)]
pub mod unix;

#[cfg(windows)]
pub mod windows;

// Re-exports
pub use channel::{IpcChannel, IpcReceiver, IpcSender};
pub use error::{IpcError, Result};
pub use file_channel::{FileChannel, FileMessage, MessageType};
pub use graceful::{
    GracefulChannel, GracefulIpcChannel, GracefulNamedPipe, GracefulWrapper, OperationGuard,
    ShutdownState,
};
pub use local_socket::{LocalSocketListener, LocalSocketStream};
pub use pipe::{AnonymousPipe, NamedPipe, PipeReader, PipeWriter};
pub use shm::SharedMemory;

// Async local socket exports (when both async and backend-interprocess features are enabled)
#[cfg(all(feature = "async", feature = "backend-interprocess"))]
pub use local_socket::{AsyncLocalSocketListener, AsyncLocalSocketStream};

// Python bindings (organized into submodules for better maintainability)
#[cfg(feature = "python-bindings")]
pub mod bindings;

#[cfg(feature = "python-bindings")]
pub use bindings::*;
