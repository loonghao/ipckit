//! Error types for ipckit

use std::io;
use thiserror::Error;

/// Result type alias for ipckit operations
pub type Result<T> = std::result::Result<T, IpcError>;

/// IPC error types
#[derive(Error, Debug)]
pub enum IpcError {
    /// I/O error from the underlying system
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// The pipe or channel is already closed
    #[error("Channel closed")]
    Closed,

    /// The pipe or channel name is invalid
    #[error("Invalid name: {0}")]
    InvalidName(String),

    /// The resource already exists
    #[error("Resource already exists: {0}")]
    AlreadyExists(String),

    /// The resource was not found
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Timeout occurred
    #[error("Operation timed out")]
    Timeout,

    /// Buffer too small
    #[error("Buffer too small: need {needed}, got {got}")]
    BufferTooSmall { needed: usize, got: usize },

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// Platform-specific error
    #[error("Platform error: {0}")]
    Platform(String),

    /// Invalid state
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Would block (for non-blocking operations)
    #[error("Operation would block")]
    WouldBlock,

    /// Other error
    #[error("{0}")]
    Other(String),
}

impl IpcError {
    /// Create a new I/O error
    pub fn io(err: io::Error) -> Self {
        Self::Io(err)
    }

    /// Create a serialization error
    pub fn serialization(msg: impl Into<String>) -> Self {
        Self::Serialization(msg.into())
    }

    /// Create a deserialization error
    pub fn deserialization(msg: impl Into<String>) -> Self {
        Self::Deserialization(msg.into())
    }

    /// Check if this is a "would block" error
    pub fn is_would_block(&self) -> bool {
        matches!(self, Self::WouldBlock)
            || matches!(self, Self::Io(e) if e.kind() == io::ErrorKind::WouldBlock)
    }

    /// Check if this is a timeout error
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout)
            || matches!(self, Self::Io(e) if e.kind() == io::ErrorKind::TimedOut)
    }
}

#[cfg(feature = "python-bindings")]
impl From<IpcError> for pyo3::PyErr {
    fn from(err: IpcError) -> pyo3::PyErr {
        use pyo3::exceptions::*;
        match err {
            IpcError::Io(e) => PyIOError::new_err(e.to_string()),
            IpcError::Closed => PyConnectionError::new_err("Channel closed"),
            IpcError::InvalidName(s) => PyValueError::new_err(s),
            IpcError::AlreadyExists(s) => PyFileExistsError::new_err(s),
            IpcError::NotFound(s) => PyFileNotFoundError::new_err(s),
            IpcError::PermissionDenied(s) => PyPermissionError::new_err(s),
            IpcError::Timeout => PyTimeoutError::new_err("Operation timed out"),
            IpcError::BufferTooSmall { needed, got } => {
                PyBufferError::new_err(format!("Buffer too small: need {needed}, got {got}"))
            }
            IpcError::Serialization(s) => PyValueError::new_err(s),
            IpcError::Deserialization(s) => PyValueError::new_err(s),
            IpcError::Platform(s) => PyOSError::new_err(s),
            IpcError::InvalidState(s) => PyRuntimeError::new_err(s),
            IpcError::WouldBlock => PyBlockingIOError::new_err("Operation would block"),
            IpcError::Other(s) => PyRuntimeError::new_err(s),
        }
    }
}
