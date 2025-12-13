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
- 🛡️ **Graceful Shutdown** - Built-in support for graceful channel shutdown
- 🔌 **Local Socket** - Unix Domain Socket / Named Pipe abstraction for cross-platform socket communication
- 🧵 **Thread Channel** - High-performance intra-process thread communication
- 📡 **Event Stream** - Real-time publish-subscribe event system
- 📋 **Task Manager** - Task lifecycle management with progress tracking
- 🌐 **Socket Server** - Multi-client socket server (like Docker's socket)

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

### Graceful Shutdown

When using IPC channels with event loops (like WebView, GUI frameworks), background threads may continue sending messages after the main event loop has closed, causing errors. The `GracefulChannel` feature solves this problem.

**Python:**
```python
import ipckit

# Create channel with graceful shutdown support
channel = ipckit.GracefulIpcChannel.create("my_channel")
channel.wait_for_client()

# ... use channel normally ...
data = channel.recv()
channel.send(b"response")

# Graceful shutdown - prevents new operations and waits for pending ones
channel.shutdown()
channel.drain()  # Wait for all pending operations to complete

# Or use shutdown with timeout (in milliseconds)
channel.shutdown_timeout(5000)  # 5 second timeout
```

**Rust:**
```rust
use ipckit::{GracefulIpcChannel, GracefulChannel};
use std::time::Duration;

fn main() -> ipckit::Result<()> {
    let mut channel = GracefulIpcChannel::<Vec<u8>>::create("my_channel")?;
    channel.wait_for_client()?;
    
    // ... use channel ...
    
    // Graceful shutdown
    channel.shutdown();
    channel.drain()?;
    
    // Or with timeout
    channel.shutdown_timeout(Duration::from_secs(5))?;
    
    Ok(())
}
```

**Key Benefits:**
- Prevents `EventLoopClosed` and similar errors
- Thread-safe shutdown signaling
- Tracks pending operations with RAII guards
- Configurable drain timeout

### Local Socket (Cross-Platform Socket Communication)

Local sockets provide a unified API for Unix Domain Sockets (Unix/macOS) and Named Pipes (Windows).

**Python Server:**
```python
import ipckit

# Create server
server = ipckit.LocalSocketListener.bind("my_socket")
print("Waiting for client...")

# Accept connection
stream = server.accept()

# Receive and send data
data = stream.read(1024)
print(f"Received: {data}")
stream.write(b"Hello from server!")

# JSON communication
json_data = stream.recv_json()
stream.send_json({"status": "ok", "message": "received"})
```

**Python Client:**
```python
import ipckit

# Connect to server
stream = ipckit.LocalSocketStream.connect("my_socket")

# Send and receive data
stream.write(b"Hello from client!")
response = stream.read(1024)
print(f"Response: {response}")

# JSON communication
stream.send_json({"action": "getData", "key": "user"})
result = stream.recv_json()
print(result)
```

**Key Benefits:**
- Cross-platform: Works on Windows, Linux, and macOS
- Bidirectional communication
- Built-in JSON serialization with length prefix
- Simple client-server model

### Thread Channel (Intra-Process Communication)

High-performance channel for communication between threads within the same process.

**Rust:**
```rust
use ipckit::ThreadChannel;
use std::thread;

fn main() {
    // Create an unbounded channel
    let (tx, rx) = ThreadChannel::<String>::unbounded();

    // Spawn producer thread
    let tx_clone = tx.clone();
    thread::spawn(move || {
        tx_clone.send("Hello from thread!".to_string()).unwrap();
    });

    // Receive in main thread
    let msg = rx.recv().unwrap();
    println!("Received: {}", msg);
}
```

### Event Stream (Publish-Subscribe)

Real-time event system for task progress, logs, and notifications.

**Rust:**
```rust
use ipckit::{EventBus, Event, EventFilter};

fn main() {
    let bus = EventBus::new(Default::default());
    let publisher = bus.publisher();

    // Subscribe to task events
    let subscriber = bus.subscribe(
        EventFilter::new().event_type("task.*")
    );

    // Publish events
    publisher.progress("task-123", 50, 100, "Half done");
    publisher.log("task-123", "info", "Processing...");

    // Receive events
    while let Some(event) = subscriber.try_recv() {
        println!("[{}] {:?}", event.event_type, event.data);
    }
}
```

### Task Manager (Task Lifecycle)

Manage long-running tasks with progress tracking and cancellation support.

**Rust:**
```rust
use ipckit::{TaskManager, TaskBuilder, TaskFilter};
use std::time::Duration;

fn main() {
    let manager = TaskManager::new(Default::default());

    // Spawn a task
    let handle = manager.spawn("Upload files", "upload", |task| {
        for i in 0..100 {
            if task.is_cancelled() {
                return;
            }
            task.set_progress(i + 1, Some(&format!("Step {}/100", i + 1)));
            std::thread::sleep(Duration::from_millis(50));
        }
        task.complete(serde_json::json!({"uploaded": 100}));
    });

    // List active tasks
    let active = manager.list(&TaskFilter::new().active());
    println!("Active tasks: {}", active.len());

    // Cancel if needed
    // manager.cancel(handle.id()).unwrap();
}
```

### Socket Server (Multi-Client Server)

Docker-style socket server for handling multiple client connections.

**Rust:**
```rust
use ipckit::{SocketServer, SocketServerConfig, Message, FnHandler};

fn main() -> ipckit::Result<()> {
    let server = SocketServer::new(SocketServerConfig::with_path("my_server"))?;

    // Handle connections with a simple function
    let handler = FnHandler::new(|conn, msg| {
        if msg.method() == Some("ping") {
            Ok(Some(Message::response(serde_json::json!({"pong": true}))))
        } else {
            Ok(None)
        }
    });

    // Run server (blocking)
    server.run(handler)?;
    Ok(())
}
```

**Client:**
```rust
use ipckit::SocketClient;

fn main() -> ipckit::Result<()> {
    let mut client = SocketClient::connect("my_server")?;

    // Send request and get response
    let result = client.request("ping", serde_json::json!({}))?;
    println!("Response: {:?}", result);

    Ok(())
}
```

## 📖 IPC Methods Comparison

| Method | Use Case | Performance | Complexity |
|--------|----------|-------------|------------|
| **Anonymous Pipe** | Parent-child processes | Fast | Low |
| **Named Pipe** | Unrelated processes | Fast | Medium |
| **Shared Memory** | Large data, frequent access | Fastest | High |
| **IPC Channel** | Message passing | Fast | Low |
| **File Channel** | Frontend-backend | Moderate | Low |
| **Graceful Channel** | Event loop integration | Fast | Low |
| **Local Socket** | Cross-platform sockets | Fast | Low |
| **Thread Channel** | Intra-process threads | Fastest | Low |
| **Event Stream** | Publish-subscribe events | Fast | Low |
| **Task Manager** | Task lifecycle | Fast | Medium |
| **Socket Server** | Multi-client server | Fast | Medium |

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
│  ┌─────────────────────────────────────────────────────────┐│
│  │              Graceful Shutdown Layer                    ││
│  │  (GracefulNamedPipe, GracefulIpcChannel, ShutdownState) ││
│  └─────────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────────┐│
│  │                  Local Socket Layer                     ││
│  │     (LocalSocketListener, LocalSocketStream)            ││
│  └─────────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────────┐│
│  │                  High-Level Services                    ││
│  │  (ThreadChannel, EventStream, TaskManager, SocketServer)││
│  └─────────────────────────────────────────────────────────┘│
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
