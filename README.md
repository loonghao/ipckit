# ipckit

[![Crates.io](https://img.shields.io/crates/v/ipckit.svg)](https://crates.io/crates/ipckit)
[![PyPI](https://img.shields.io/pypi/v/ipckit.svg)](https://pypi.org/project/ipckit/)
[![Documentation](https://docs.rs/ipckit/badge.svg)](https://docs.rs/ipckit)
[![CI](https://github.com/loonghao/ipckit/actions/workflows/ci.yml/badge.svg)](https://github.com/loonghao/ipckit/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

Local IPC primitives for Rust and Python. The same package covers pipes, shared memory, local sockets, message channels, file-backed frontend integration, events, tasks, and CLI progress.

[中文文档](README_zh.md)

## Install

### Python

~~~bash
python -m pip install ipckit
~~~

Prebuilt wheels are available for CPython 3.8 and newer through the stable ABI. CPython 3.7 hosts use the native
`cp37-cp37m` wheel on Linux x86_64 and Windows x64. The `py37-lite` wheel contains type stubs only for other
CPython 3.7 environments. Maya 2022 and Blender 2.83 use the native CPython 3.7 wheel.

### Rust

~~~toml
[dependencies]
ipckit = "0.1"
~~~

### CLI

~~~bash
cargo install --path crates/ipckit-cli
~~~

## Choose a transport

| Use case | API | Transport |
| --- | --- | --- |
| Parent and child processes | `AnonymousPipe` | Anonymous pipe |
| Two unrelated processes | `NamedPipe` or `IpcChannel` | Named pipe on Windows |
| Large shared data | `SharedMemory` | Memory-mapped region |
| A local request/response socket | `LocalSocketListener` and `LocalSocketStream` | Unix socket or named pipe |
| A web frontend and a local backend | `FileChannel` | JSON files with locking |
| In-process workers | `ThreadChannel` | Thread channel |
| Events and progress | `EventBus` | In-process event stream |
| Long-running work | `TaskManager` | Task state and cancellation |
| CLI progress reporting | `CliBridge` or `wrap_command` | Socket-based task updates |

There is no background service to install. The process that creates a channel owns the server side.

## Quick start

### Python process pair

Start the server first.

~~~python
# server.py
import ipckit

channel = ipckit.IpcChannel.create("demo")
channel.wait_for_client()

request = channel.recv_json()
channel.send_json({"ok": True, "echo": request})
~~~

Connect from another process.

~~~python
# client.py
import ipckit

channel = ipckit.IpcChannel.connect("demo")
channel.send_json({"message": "hello"})
print(channel.recv_json())
~~~

Use `NamedPipe` when you need raw bytes. Use `IpcChannel` when messages are JSON values.

### Rust typed messages

~~~rust
use ipckit::IpcChannel;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    text: String,
}

fn main() -> ipckit::Result<()> {
    let mut channel = IpcChannel::<Message>::create("demo")?;
    channel.wait_for_client()?;

    let message = channel.recv()?;
    println!("{}", message.text);
    channel.send(&Message {
        text: "reply".into(),
    })?;

    Ok(())
}
~~~

The client uses `IpcChannel::<Message>::connect("demo")`, then calls `send` and `recv`.

## Frontend and backend integration

`FileChannel` is useful when a desktop WebView or browser frontend can read and write a shared directory.

~~~python
# backend.py
import ipckit

channel = ipckit.FileChannel.backend("./ipc-channel")
request_id = channel.send_request("getData", {"key": "user_info"})
response = channel.wait_response(request_id, timeout_ms=5000)
print(response)
channel.send_event("status", {"ready": True})
~~~

The backend writes to `backend_to_frontend.json`. The frontend writes to `frontend_to_backend.json`. Both files contain JSON message arrays and are updated with file locking.

~~~javascript
const response = await fetch("./ipc-channel/backend_to_frontend.json");
const messages = await response.json();
~~~

Use `FileChannel.frontend("./ipc-channel")` for a Python frontend process. See [examples/frontend](examples/frontend) for a complete backend, file frontend, and WebView example.

## Local sockets

The local socket API keeps the application code the same across Unix and Windows.

~~~python
# server.py
import ipckit

listener = ipckit.LocalSocketListener.bind("demo-socket")
stream = listener.accept()
print(stream.recv_json())
stream.send_json({"status": "ok"})
~~~

~~~python
# client.py
import ipckit

stream = ipckit.LocalSocketStream.connect("demo-socket")
stream.send_json({"action": "ping"})
print(stream.recv_json())
~~~

Use `send_json` and `recv_json` for framed JSON messages. Use `read_exact` and `write_all` for a byte protocol.

## Events and tasks

Publish progress or log events without introducing another service.

~~~python
import ipckit

bus = ipckit.EventBus()
subscriber = bus.subscribe(ipckit.EventFilter().event_type("task.*"))
publisher = bus.publisher()

publisher.progress("task-1", 1, 2, "running")
event = subscriber.recv_timeout(1000)
print(event.event_type, event.data)
~~~

Track cancellable work with `TaskManager`.

~~~python
import ipckit

manager = ipckit.TaskManager()
task = manager.create_task("index", "build")
task.start()
task.set_progress(50, "halfway")

if task.is_cancelled:
    task.fail("cancelled")
else:
    task.complete({"files": 10})
~~~

## CLI integration

Wrap an existing command and collect its output.

~~~python
import ipckit

result = ipckit.wrap_command(
    ["cargo", "build", "--release"],
    task_name="Build",
    task_type="build",
)
print(result.exit_code, result.duration_ms)
~~~

Use `CliBridge` when the host already exposes an ipckit task server.

~~~python
import ipckit

with ipckit.CliBridge.connect() as bridge:
    bridge.register_task("Build", "build")
    bridge.set_progress(50, "compiling")
    bridge.complete({"ok": True})
~~~

Progress can be parsed from command output with `parse_progress(line)`.

## Main-thread host integration

DCC and game-engine hosts often require UI work on their main thread. Rust callers can dispatch work from a worker and drain it from the host idle callback.

~~~rust
use ipckit::MainThreadPump;
use std::time::Duration;

let pump = MainThreadPump::new();

pump.dispatch(|| {
    // Call the host UI API here.
});

// Call this from the host's idle or update callback.
let stats = pump.pump(Duration::from_millis(8));
println!("processed: {}", stats.processed);
~~~

Attach `ThreadAffinity::Main` to task builders when the task metadata must record main-thread affinity. The host remains responsible for calling `pump`.

## CLI commands

~~~bash
ipckit create --type pipe --name demo
ipckit listen --type pipe --name demo
ipckit send --type pipe --name demo "hello"

ipckit monitor --format json
ipckit generate client --type pipe --name demo --output client.rs
~~~

Run `ipckit --help` for all commands and options.

## Windows application-control policy

If importing the Windows wheel reports that an application-control policy blocked the native `.pyd`, inspect the CodeIntegrity Operational log for event 3077 and the AppLocker EXE and DLL log for event 8004. Deploy the package through an approved path or installer, add an allow rule, or sign the `.pyd` with a certificate trusted by the host policy.

~~~powershell
$pyd = Get-ChildItem "D:\path\to\site-packages\ipckit\*.pyd"
Get-AuthenticodeSignature $pyd.FullName
~~~

Moving the file or removing Mark-of-the-Web metadata does not replace an App Control allow rule.

## Build and test

~~~bash
python -m pip install maturin pytest pytest-timeout
maturin develop --release
python -m pytest tests
cargo test --workspace
~~~

The Python extension is built with the `python-bindings`, `ext-module`, and `abi3-py38` features.

## Project links

- [Rust API documentation](https://docs.rs/ipckit)
- [Python type stubs](python/ipckit/__init__.pyi)
- [Examples](examples)
- [Changelog](CHANGELOG.md)

## License

MIT or Apache-2.0.
