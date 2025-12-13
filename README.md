# ipckit

[![Crates.io](https://img.shields.io/crates/v/ipckit.svg)](https://crates.io/crates/ipckit)
[![PyPI](https://img.shields.io/pypi/v/ipckit.svg)](https://pypi.org/project/ipckit/)
[![Documentation](https://docs.rs/ipckit/badge.svg)](https://docs.rs/ipckit)
[![CI](https://github.com/loonghao/ipckit/actions/workflows/ci.yml/badge.svg)](https://github.com/loonghao/ipckit/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Python Versions](https://img.shields.io/pypi/pyversions/ipckit.svg)](https://pypi.org/project/ipckit/)
[![Rust Version](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Downloads](https://img.shields.io/pypi/dm/ipckit.svg)](https://pypi.org/project/ipckit/)

A high-performance, cross-platform IPC (Inter-Process Communication) library for Rust and Python, powered by Rust.

[中文文档](README_zh.md)

## ✨ Features

- 🚀 **High Performance** - Written in Rust, with zero-copy where possible
- 🔀 **Cross-Platform** - Works on Windows, Linux, and macOS
- 🐍 **Python Bindings** - First-class Python support via PyO3
- 📦 **Multiple IPC Methods** - Pipes, Shared Memory, Channels, and File-based IPC
- 🔒 **Thread-Safe** - Safe concurrent access across processes
- ⚡ **Native JSON** - Built-in fast JSON serialization using Rust's serde_json

## 📦 Installation

### Python

```bash
pip install ipckit
```

### Rust

```toml
[dependencies]
ipckit = "0.1"
```

## 🚀 Quick Start

### Anonymous Pipe (Parent-Child Communication)

**Python:**
```python
import ipckit
import subprocess

# Create pipe pair
pipe = ipckit.AnonymousPipe()

# Write to pipe
pipe.write(b"Hello from parent!")

# Read from pipe
data = pipe.read(1024)
print(data)
```

**Rust:**
```rust
use ipckit::AnonymousPipe;

fn main() -> ipckit::Result<()> {
    let pipe = AnonymousPipe::new()?;
    
    pipe.write_all(b"Hello from Rust!")?;
    
    let mut buf = [0u8; 1024];
    let n = pipe.read(&mut buf)?;
    println!("{}", String::from_utf8_lossy(&buf[..n]));
    
    Ok(())
}
```

### Named Pipe (Unrelated Process Communication)

**Python Server:**
```python
import ipckit

# Create server
server = ipckit.NamedPipe.create("my_pipe")
print("Waiting for client...")
server.wait_for_client()

# Communicate
data = server.read(1024)
server.write(b"Response from server")
```

**Python Client:**
```python
import ipckit

# Connect to server
client = ipckit.NamedPipe.connect("my_pipe")

# Communicate
client.write(b"Hello from client")
response = client.read(1024)
print(response)
```

### Shared Memory (Fast Data Exchange)

**Python:**
```python
import ipckit

# Create shared memory (owner)
shm = ipckit.SharedMemory.create("my_shm", 4096)
shm.write(0, b"Shared data here!")

# In another process, open existing
shm2 = ipckit.SharedMemory.open("my_shm")
data = shm2.read(0, 17)
print(data)  # b"Shared data here!"
```

**Rust:**
```rust
use ipckit::SharedMemory;

fn main() -> ipckit::Result<()> {
    // Create
    let shm = SharedMemory::create("my_shm", 4096)?;
    shm.write(0, b"Hello from Rust!")?;
    
    // Open in another process
    let shm2 = SharedMemory::open("my_shm")?;
    let data = shm2.read(0, 16)?;
    
    Ok(())
}
```

### IPC Channel (High-Level Message Passing)

**Python:**
```python
import ipckit

# Server
channel = ipckit.IpcChannel.create("my_channel")
channel.wait_for_client()

# Send/receive JSON
channel.send_json({"type": "greeting", "message": "Hello!"})
response = channel.recv_json()
print(response)
```

### File Channel (Frontend-Backend Communication)

Perfect for desktop applications where Python backend communicates with web frontend.

**Python Backend:**
```python
import ipckit

# Create backend channel
channel = ipckit.FileChannel.backend("./ipc_channel")

# Send request to frontend
request_id = channel.send_request("getData", {"key": "user_info"})

# Wait for response
response = channel.wait_response(request_id, timeout_ms=5000)
print(response)

# Send events
channel.send_event("status_update", {"status": "ready"})
```

**JavaScript Frontend:**
```javascript
// Read from: ./ipc_channel/backend_to_frontend.json
// Write to:  ./ipc_channel/frontend_to_backend.json

async function pollMessages() {
    const response = await fetch('./ipc_channel/backend_to_frontend.json');
    const messages = await response.json();
    // Process new messages...
}
```

### Native JSON Functions

ipckit provides Rust-native JSON functions that are faster than Python's built-in json module:

```python
import ipckit

# Serialize (1.2x faster than json.dumps)
data = {"name": "test", "values": [1, 2, 3]}
json_str = ipckit.json_dumps(data)

# Pretty print
pretty_str = ipckit.json_dumps_pretty(data)

# Deserialize
obj = ipckit.json_loads('{"key": "value"}')
```

## 📖 IPC Methods Comparison

| Method | Use Case | Performance | Complexity |
|--------|----------|-------------|------------|
| **Anonymous Pipe** | Parent-child processes | Fast | Low |
| **Named Pipe** | Unrelated processes | Fast | Medium |
| **Shared Memory** | Large data, frequent access | Fastest | High |
| **IPC Channel** | Message passing | Fast | Low |
| **File Channel** | Frontend-backend | Moderate | Low |

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Python Application                       │
├─────────────────────────────────────────────────────────────┤
│                    ipckit Python Bindings                    │
│                         (PyO3)                               │
├─────────────────────────────────────────────────────────────┤
│                     ipckit Rust Core                         │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────────────────┐│
│  │  Pipes  │ │   SHM   │ │ Channel │ │    File Channel     ││
│  └─────────┘ └─────────┘ └─────────┘ └─────────────────────┘│
├─────────────────────────────────────────────────────────────┤
│              Platform Abstraction Layer                      │
│         (Windows / Linux / macOS)                            │
└─────────────────────────────────────────────────────────────┘
```

## 🔧 Building from Source

### Prerequisites

- Rust 1.70+
- Python 3.7+
- maturin (`pip install maturin`)

### Build

```bash
# Clone repository
git clone https://github.com/loonghao/ipckit.git
cd ipckit

# Build Python package
maturin develop --release

# Run tests
pytest tests/
cargo test
```

## 📝 License

This project is dual-licensed under:

- [MIT License](LICENSE-MIT)
- [Apache License 2.0](LICENSE-APACHE)

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📚 Documentation

- [API Documentation (Rust)](https://docs.rs/ipckit)
- [API Documentation (Python)](https://github.com/loonghao/ipckit/wiki)
- [Examples](examples/)

## 🙏 Acknowledgments

- [PyO3](https://pyo3.rs/) - Rust bindings for Python
- [maturin](https://www.maturin.rs/) - Build and publish Rust-based Python packages
- [serde](https://serde.rs/) - Serialization framework for Rust
