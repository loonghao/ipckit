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

pub mod error;
pub mod pipe;
pub mod shm;
pub mod channel;
pub mod file_channel;

#[cfg(unix)]
pub mod unix;

#[cfg(windows)]
pub mod windows;

// Re-exports
pub use error::{IpcError, Result};
pub use pipe::{AnonymousPipe, NamedPipe, PipeReader, PipeWriter};
pub use shm::SharedMemory;
pub use channel::{IpcChannel, IpcSender, IpcReceiver};
pub use file_channel::{FileChannel, FileMessage, MessageType};

// Python bindings
#[cfg(feature = "python-bindings")]
pub mod python;

#[cfg(feature = "python-bindings")]
pub use python::*;
