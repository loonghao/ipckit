//! High-level message channel for IPC
//!
//! Provides a typed message passing interface with automatic serialization.

use crate::error::{IpcError, Result};
use crate::pipe::NamedPipe;
use serde::{de::DeserializeOwned, Serialize};
use std::io::{Read, Write};
use std::marker::PhantomData;

/// Message header size (4 bytes for length)
const HEADER_SIZE: usize = 4;

/// Maximum message size (16 MB)
const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

/// IPC channel for bidirectional message passing
pub struct IpcChannel<T = Vec<u8>> {
    pipe: NamedPipe,
    _marker: PhantomData<T>,
}

/// Sender end of an IPC channel
pub struct IpcSender<T = Vec<u8>> {
    pipe: NamedPipe,
    _marker: PhantomData<T>,
}

/// Receiver end of an IPC channel
pub struct IpcReceiver<T = Vec<u8>> {
    pipe: NamedPipe,
    _marker: PhantomData<T>,
}

impl<T> IpcChannel<T> {
    /// Create a new IPC channel server
    pub fn create(name: &str) -> Result<Self> {
        let pipe = NamedPipe::create(name)?;
        Ok(Self {
            pipe,
            _marker: PhantomData,
        })
    }

    /// Connect to an existing IPC channel
    pub fn connect(name: &str) -> Result<Self> {
        let pipe = NamedPipe::connect(name)?;
        Ok(Self {
            pipe,
            _marker: PhantomData,
        })
    }

    /// Get the channel name
    pub fn name(&self) -> &str {
        self.pipe.name()
    }

    /// Check if this is the server end
    pub fn is_server(&self) -> bool {
        self.pipe.is_server()
    }

    /// Wait for a client to connect (server only)
    pub fn wait_for_client(&self) -> Result<()> {
        self.pipe.wait_for_client()
    }
}

impl IpcChannel<Vec<u8>> {
    /// Send raw bytes
    pub fn send_bytes(&mut self, data: &[u8]) -> Result<()> {
        if data.len() > MAX_MESSAGE_SIZE {
            return Err(IpcError::BufferTooSmall {
                needed: data.len(),
                got: MAX_MESSAGE_SIZE,
            });
        }

        // Write length header
        let len = data.len() as u32;
        self.pipe.write_all(&len.to_le_bytes())?;

        // Write data
        self.pipe.write_all(data)?;
        Ok(())
    }

    /// Receive raw bytes
    pub fn recv_bytes(&mut self) -> Result<Vec<u8>> {
        // Read length header
        let mut header = [0u8; HEADER_SIZE];
        self.pipe.read_exact(&mut header)?;
        let len = u32::from_le_bytes(header) as usize;

        if len > MAX_MESSAGE_SIZE {
            return Err(IpcError::BufferTooSmall {
                needed: len,
                got: MAX_MESSAGE_SIZE,
            });
        }

        // Read data
        let mut data = vec![0u8; len];
        self.pipe.read_exact(&mut data)?;
        Ok(data)
    }
}

impl<T: Serialize + DeserializeOwned> IpcChannel<T> {
    /// Send a typed message (serialized as JSON)
    pub fn send(&mut self, msg: &T) -> Result<()> {
        let data = serde_json::to_vec(msg).map_err(|e| IpcError::serialization(e.to_string()))?;
        self.send_raw(&data)
    }

    /// Receive a typed message (deserialized from JSON)
    pub fn recv(&mut self) -> Result<T> {
        let data = self.recv_raw()?;
        serde_json::from_slice(&data).map_err(|e| IpcError::deserialization(e.to_string()))
    }

    /// Send raw bytes (internal)
    fn send_raw(&mut self, data: &[u8]) -> Result<()> {
        if data.len() > MAX_MESSAGE_SIZE {
            return Err(IpcError::BufferTooSmall {
                needed: data.len(),
                got: MAX_MESSAGE_SIZE,
            });
        }

        let len = data.len() as u32;
        self.pipe.write_all(&len.to_le_bytes())?;
        self.pipe.write_all(data)?;
        Ok(())
    }

    /// Receive raw bytes (internal)
    fn recv_raw(&mut self) -> Result<Vec<u8>> {
        let mut header = [0u8; HEADER_SIZE];
        self.pipe.read_exact(&mut header)?;
        let len = u32::from_le_bytes(header) as usize;

        if len > MAX_MESSAGE_SIZE {
            return Err(IpcError::BufferTooSmall {
                needed: len,
                got: MAX_MESSAGE_SIZE,
            });
        }

        let mut data = vec![0u8; len];
        self.pipe.read_exact(&mut data)?;
        Ok(data)
    }
}

impl<T> IpcSender<T> {
    /// Create a new sender from a named pipe
    pub fn new(pipe: NamedPipe) -> Self {
        Self {
            pipe,
            _marker: PhantomData,
        }
    }

    /// Create a sender that connects to an existing channel
    pub fn connect(name: &str) -> Result<Self> {
        let pipe = NamedPipe::connect(name)?;
        Ok(Self::new(pipe))
    }
}

impl IpcSender<Vec<u8>> {
    /// Send raw bytes
    pub fn send_bytes(&mut self, data: &[u8]) -> Result<()> {
        if data.len() > MAX_MESSAGE_SIZE {
            return Err(IpcError::BufferTooSmall {
                needed: data.len(),
                got: MAX_MESSAGE_SIZE,
            });
        }

        let len = data.len() as u32;
        self.pipe.write_all(&len.to_le_bytes())?;
        self.pipe.write_all(data)?;
        Ok(())
    }
}

impl<T: Serialize> IpcSender<T> {
    /// Send a typed message
    pub fn send(&mut self, msg: &T) -> Result<()> {
        let data = serde_json::to_vec(msg).map_err(|e| IpcError::serialization(e.to_string()))?;

        if data.len() > MAX_MESSAGE_SIZE {
            return Err(IpcError::BufferTooSmall {
                needed: data.len(),
                got: MAX_MESSAGE_SIZE,
            });
        }

        let len = data.len() as u32;
        self.pipe.write_all(&len.to_le_bytes())?;
        self.pipe.write_all(&data)?;
        Ok(())
    }
}

impl<T> IpcReceiver<T> {
    /// Create a new receiver from a named pipe
    pub fn new(pipe: NamedPipe) -> Self {
        Self {
            pipe,
            _marker: PhantomData,
        }
    }

    /// Create a receiver that creates a new channel
    pub fn create(name: &str) -> Result<Self> {
        let pipe = NamedPipe::create(name)?;
        Ok(Self::new(pipe))
    }

    /// Wait for a sender to connect
    pub fn wait_for_sender(&self) -> Result<()> {
        self.pipe.wait_for_client()
    }
}

impl IpcReceiver<Vec<u8>> {
    /// Receive raw bytes
    pub fn recv_bytes(&mut self) -> Result<Vec<u8>> {
        let mut header = [0u8; HEADER_SIZE];
        self.pipe.read_exact(&mut header)?;
        let len = u32::from_le_bytes(header) as usize;

        if len > MAX_MESSAGE_SIZE {
            return Err(IpcError::BufferTooSmall {
                needed: len,
                got: MAX_MESSAGE_SIZE,
            });
        }

        let mut data = vec![0u8; len];
        self.pipe.read_exact(&mut data)?;
        Ok(data)
    }
}

impl<T: DeserializeOwned> IpcReceiver<T> {
    /// Receive a typed message
    pub fn recv(&mut self) -> Result<T> {
        let mut header = [0u8; HEADER_SIZE];
        self.pipe.read_exact(&mut header)?;
        let len = u32::from_le_bytes(header) as usize;

        if len > MAX_MESSAGE_SIZE {
            return Err(IpcError::BufferTooSmall {
                needed: len,
                got: MAX_MESSAGE_SIZE,
            });
        }

        let mut data = vec![0u8; len];
        self.pipe.read_exact(&mut data)?;

        serde_json::from_slice(&data).map_err(|e| IpcError::deserialization(e.to_string()))
    }
}

/// Create a pair of connected IPC sender and receiver
///
/// Returns (sender, receiver) where sender can send messages and receiver can receive them.
pub fn channel<T>(name: &str) -> Result<(IpcSender<T>, IpcReceiver<T>)> {
    let receiver = IpcReceiver::create(name)?;
    let sender = IpcSender::connect(name)?;
    Ok((sender, receiver))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::thread;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[allow(dead_code)]
    struct TestMessage {
        id: u32,
        content: String,
    }

    #[test]
    fn test_channel_bytes() {
        let name = format!("test_channel_{}", std::process::id());

        let handle = thread::spawn({
            let name = name.clone();
            move || {
                let mut channel = IpcChannel::<Vec<u8>>::create(&name).unwrap();
                channel.wait_for_client().ok();
                let data = channel.recv_bytes().unwrap();
                assert_eq!(data, b"Hello, IPC!");
            }
        });

        // Give server time to start
        thread::sleep(std::time::Duration::from_millis(100));

        let mut client = IpcChannel::<Vec<u8>>::connect(&name).unwrap();
        client.send_bytes(b"Hello, IPC!").unwrap();

        handle.join().unwrap();
    }
}
