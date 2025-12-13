//! File-based IPC Channel
//!
//! A simple IPC mechanism using local files for communication.
//! This is useful for frontend-backend communication where:
//! - Backend (Python) writes to a file, Frontend reads it
//! - Frontend writes to another file, Backend reads it
//!
//! ## Protocol
//!
//! Each message file contains:
//! - Line 1: Message ID (UUID)
//! - Line 2: Timestamp (Unix epoch in milliseconds)
//! - Line 3: Message type (request/response/event)
//! - Line 4+: JSON payload
//!
//! ## File Structure
//!
//! ```text
//! {channel_dir}/
//! ├── backend_to_frontend.json   # Backend writes, Frontend reads
//! ├── frontend_to_backend.json   # Frontend writes, Backend reads
//! ├── backend_to_frontend.lock   # Lock file for atomic writes
//! ├── frontend_to_backend.lock   # Lock file for atomic writes
//! └── .channel_info              # Channel metadata
//! ```

use crate::error::{IpcError, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Message types for file-based IPC
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    Request,
    Response,
    Event,
}

/// A message in the file-based IPC protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMessage {
    /// Unique message ID
    pub id: String,
    /// Timestamp in milliseconds since Unix epoch
    pub timestamp: u64,
    /// Message type
    #[serde(rename = "type")]
    pub msg_type: MessageType,
    /// For responses, the ID of the request being responded to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    /// Method name (for requests)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// Message payload (JSON value)
    pub payload: serde_json::Value,
    /// Error message (for error responses)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl FileMessage {
    /// Create a new request message
    pub fn request(method: &str, payload: serde_json::Value) -> Self {
        Self {
            id: uuid_v4(),
            timestamp: current_timestamp_ms(),
            msg_type: MessageType::Request,
            reply_to: None,
            method: Some(method.to_string()),
            payload,
            error: None,
        }
    }

    /// Create a response message
    pub fn response(request_id: &str, payload: serde_json::Value) -> Self {
        Self {
            id: uuid_v4(),
            timestamp: current_timestamp_ms(),
            msg_type: MessageType::Response,
            reply_to: Some(request_id.to_string()),
            method: None,
            payload,
            error: None,
        }
    }

    /// Create an error response
    pub fn error_response(request_id: &str, error: &str) -> Self {
        Self {
            id: uuid_v4(),
            timestamp: current_timestamp_ms(),
            msg_type: MessageType::Response,
            reply_to: Some(request_id.to_string()),
            method: None,
            payload: serde_json::Value::Null,
            error: Some(error.to_string()),
        }
    }

    /// Create an event message (no response expected)
    pub fn event(name: &str, payload: serde_json::Value) -> Self {
        Self {
            id: uuid_v4(),
            timestamp: current_timestamp_ms(),
            msg_type: MessageType::Event,
            reply_to: None,
            method: Some(name.to_string()),
            payload,
            error: None,
        }
    }
}

/// File-based IPC channel for backend (Python/Rust) side
pub struct FileChannel {
    /// Channel directory
    dir: PathBuf,
    /// File for outgoing messages (backend -> frontend)
    outbox_path: PathBuf,
    /// File for incoming messages (frontend -> backend)
    inbox_path: PathBuf,
    /// Last processed message ID from inbox
    last_inbox_id: Option<String>,
    /// Last processed message timestamp
    last_inbox_timestamp: u64,
}

impl FileChannel {
    /// Create or open a file channel
    ///
    /// # Arguments
    /// * `dir` - Directory for channel files (will be created if not exists)
    /// * `is_backend` - True for backend side, false for frontend side
    pub fn new<P: AsRef<Path>>(dir: P, is_backend: bool) -> Result<Self> {
        let dir = dir.as_ref().to_path_buf();

        // Create directory if not exists
        fs::create_dir_all(&dir)?;

        // Determine file paths based on role
        let (outbox_path, inbox_path) = if is_backend {
            (
                dir.join("backend_to_frontend.json"),
                dir.join("frontend_to_backend.json"),
            )
        } else {
            (
                dir.join("frontend_to_backend.json"),
                dir.join("backend_to_frontend.json"),
            )
        };

        // Create channel info file
        let info_path = dir.join(".channel_info");
        if !info_path.exists() {
            let info = serde_json::json!({
                "version": "1.0",
                "created": current_timestamp_ms(),
                "protocol": "file-ipc"
            });
            fs::write(&info_path, serde_json::to_string_pretty(&info).unwrap())?;
        }

        // Initialize empty message files if not exist
        for path in [&outbox_path, &inbox_path] {
            if !path.exists() {
                fs::write(path, "[]")?;
            }
        }

        Ok(Self {
            dir,
            outbox_path,
            inbox_path,
            last_inbox_id: None,
            last_inbox_timestamp: 0,
        })
    }

    /// Create a backend-side channel
    pub fn backend<P: AsRef<Path>>(dir: P) -> Result<Self> {
        Self::new(dir, true)
    }

    /// Create a frontend-side channel
    pub fn frontend<P: AsRef<Path>>(dir: P) -> Result<Self> {
        Self::new(dir, false)
    }

    /// Get the channel directory
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Send a message (write to outbox)
    pub fn send(&self, message: &FileMessage) -> Result<()> {
        let lock_path = self.outbox_path.with_extension("lock");
        let _lock = FileLock::acquire(&lock_path)?;

        // Read existing messages
        let mut messages = self.read_message_file(&self.outbox_path)?;

        // Add new message
        messages.push(message.clone());

        // Keep only recent messages (last 100)
        if messages.len() > 100 {
            let skip_count = messages.len() - 100;
            messages = messages.into_iter().skip(skip_count).collect();
        }

        // Write back atomically
        let temp_path = self.outbox_path.with_extension("tmp");
        let content = serde_json::to_string_pretty(&messages)
            .map_err(|e| IpcError::serialization(e.to_string()))?;
        fs::write(&temp_path, &content)?;
        fs::rename(&temp_path, &self.outbox_path)?;

        Ok(())
    }

    /// Send a request and return the message ID
    pub fn send_request(&self, method: &str, params: serde_json::Value) -> Result<String> {
        let msg = FileMessage::request(method, params);
        let id = msg.id.clone();
        self.send(&msg)?;
        Ok(id)
    }

    /// Send a response to a request
    pub fn send_response(&self, request_id: &str, result: serde_json::Value) -> Result<()> {
        let msg = FileMessage::response(request_id, result);
        self.send(&msg)
    }

    /// Send an error response
    pub fn send_error(&self, request_id: &str, error: &str) -> Result<()> {
        let msg = FileMessage::error_response(request_id, error);
        self.send(&msg)
    }

    /// Send an event
    pub fn send_event(&self, name: &str, payload: serde_json::Value) -> Result<()> {
        let msg = FileMessage::event(name, payload);
        self.send(&msg)
    }

    /// Receive new messages from inbox
    pub fn recv(&mut self) -> Result<Vec<FileMessage>> {
        let messages = self.read_message_file(&self.inbox_path)?;

        // Filter to only new messages
        let new_messages: Vec<FileMessage> = messages
            .into_iter()
            .filter(|m| {
                m.timestamp > self.last_inbox_timestamp
                    || (m.timestamp == self.last_inbox_timestamp
                        && self.last_inbox_id.as_ref() != Some(&m.id))
            })
            .collect();

        // Update last processed
        if let Some(last) = new_messages.last() {
            self.last_inbox_timestamp = last.timestamp;
            self.last_inbox_id = Some(last.id.clone());
        }

        Ok(new_messages)
    }

    /// Receive a single new message (non-blocking)
    pub fn recv_one(&mut self) -> Result<Option<FileMessage>> {
        let messages = self.recv()?;
        Ok(messages.into_iter().next())
    }

    /// Wait for a response to a specific request
    pub fn wait_response(&mut self, request_id: &str, timeout: Duration) -> Result<FileMessage> {
        let start = std::time::Instant::now();
        let poll_interval = Duration::from_millis(50);

        loop {
            let messages = self.recv()?;

            for msg in messages {
                if msg.msg_type == MessageType::Response
                    && msg.reply_to.as_ref() == Some(&request_id.to_string())
                {
                    return Ok(msg);
                }
            }

            if start.elapsed() > timeout {
                return Err(IpcError::Timeout);
            }

            std::thread::sleep(poll_interval);
        }
    }

    /// Poll for new messages with a callback
    pub fn poll<F>(&mut self, interval: Duration, mut callback: F) -> Result<()>
    where
        F: FnMut(FileMessage) -> bool,
    {
        loop {
            let messages = self.recv()?;

            for msg in messages {
                if !callback(msg) {
                    return Ok(());
                }
            }

            std::thread::sleep(interval);
        }
    }

    /// Clear all messages in both inbox and outbox
    pub fn clear(&self) -> Result<()> {
        fs::write(&self.outbox_path, "[]")?;
        fs::write(&self.inbox_path, "[]")?;
        Ok(())
    }

    /// Read messages from a file
    fn read_message_file(&self, path: &Path) -> Result<Vec<FileMessage>> {
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(path)?;
        if content.trim().is_empty() || content.trim() == "[]" {
            return Ok(Vec::new());
        }

        serde_json::from_str(&content).map_err(|e| IpcError::deserialization(e.to_string()))
    }
}

/// Simple file-based lock for atomic operations
struct FileLock {
    path: PathBuf,
}

impl FileLock {
    fn acquire(path: &Path) -> Result<Self> {
        let path = path.to_path_buf();
        let max_attempts = 50;
        let wait_time = Duration::from_millis(10);

        for _ in 0..max_attempts {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut file) => {
                    // Write PID to lock file
                    let _ = writeln!(file, "{}", std::process::id());
                    return Ok(Self { path });
                }
                Err(_) => {
                    std::thread::sleep(wait_time);
                }
            }
        }

        // Force acquire if lock is stale (older than 5 seconds)
        if let Ok(metadata) = fs::metadata(&path) {
            if let Ok(modified) = metadata.modified() {
                if modified.elapsed().unwrap_or_default() > Duration::from_secs(5) {
                    let _ = fs::remove_file(&path);
                    return Self::acquire(&path);
                }
            }
        }

        Err(IpcError::Timeout)
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Generate a simple UUID v4
fn uuid_v4() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    let state = RandomState::new();
    let mut hasher = state.build_hasher();
    hasher.write_u64(current_timestamp_ms());
    hasher.write_usize(std::process::id() as usize);
    let h1 = hasher.finish();

    let state2 = RandomState::new();
    let mut hasher2 = state2.build_hasher();
    hasher2.write_u64(h1);
    let h2 = hasher2.finish();

    format!(
        "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        (h1 >> 32) as u32,
        (h1 >> 16) as u16,
        h1 as u16 & 0x0FFF,
        (h2 >> 48) as u16 & 0x3FFF | 0x8000,
        h2 & 0xFFFFFFFFFFFF
    )
}

/// Get current timestamp in milliseconds
fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use tempfile::tempdir;

    #[test]
    fn test_file_channel_basic() {
        let dir = tempdir().unwrap();

        let mut backend = FileChannel::backend(dir.path()).unwrap();
        let mut frontend = FileChannel::frontend(dir.path()).unwrap();

        // Backend sends request
        let msg = FileMessage::request("ping", serde_json::json!({}));
        backend.send(&msg).unwrap();

        // Frontend receives
        let received = frontend.recv().unwrap();
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].method.as_ref().unwrap(), "ping");

        // Frontend sends response
        frontend
            .send_response(&received[0].id, serde_json::json!({"pong": true}))
            .unwrap();

        // Backend receives response
        let responses = backend.recv().unwrap();
        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].reply_to.as_ref().unwrap(), &received[0].id);
    }

    #[test]
    fn test_file_channel_concurrent() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path().to_path_buf();

        let handle = thread::spawn({
            let dir_path = dir_path.clone();
            move || {
                let mut frontend = FileChannel::frontend(&dir_path).unwrap();
                thread::sleep(Duration::from_millis(100));

                // Wait for request
                loop {
                    let msgs = frontend.recv().unwrap();
                    for msg in msgs {
                        if msg.method.as_ref() == Some(&"test".to_string()) {
                            frontend
                                .send_response(&msg.id, serde_json::json!({"ok": true}))
                                .unwrap();
                            return;
                        }
                    }
                    thread::sleep(Duration::from_millis(50));
                }
            }
        });

        let mut backend = FileChannel::backend(&dir_path).unwrap();
        let request_id = backend
            .send_request("test", serde_json::json!({"value": 42}))
            .unwrap();

        let response = backend
            .wait_response(&request_id, Duration::from_secs(5))
            .unwrap();
        assert!(response.payload.get("ok").unwrap().as_bool().unwrap());

        handle.join().unwrap();
    }
}
