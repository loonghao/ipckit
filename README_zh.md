# ipckit

[![Crates.io](https://img.shields.io/crates/v/ipckit.svg)](https://crates.io/crates/ipckit)
[![PyPI](https://img.shields.io/pypi/v/ipckit.svg)](https://pypi.org/project/ipckit/)
[![Documentation](https://docs.rs/ipckit/badge.svg)](https://docs.rs/ipckit)
[![CI](https://github.com/loonghao/ipckit/actions/workflows/ci.yml/badge.svg)](https://github.com/loonghao/ipckit/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

ipckit 为 Rust 和 Python 提供本地进程间通信能力，包含管道、共享内存、本地套接字、消息通道、前后端文件通信、事件、任务和 CLI 进度上报。

[English](README.md)

## 安装

### Python

~~~bash
python -m pip install ipckit
~~~

预构建 wheel 通过稳定 ABI 支持 CPython 3.8 及以上版本。CPython 3.7 宿主在 Linux x86_64 和 Windows x64 上使用原生
`cp37-cp37m` wheel。其他 CPython 3.7 环境使用仅包含类型存根的 `py37-lite` wheel。Maya 2022 和 Blender 2.83 使用原生 CPython 3.7 wheel。

### Rust

~~~toml
[dependencies]
ipckit = "0.1"
~~~

### CLI

~~~bash
cargo install --path crates/ipckit-cli
~~~

## 选择通信方式

| 场景 | API | 传输方式 |
| --- | --- | --- |
| 父子进程 | `AnonymousPipe` | 匿名管道 |
| 两个无关进程 | `NamedPipe` 或 `IpcChannel` | Windows 使用命名管道 |
| 大块共享数据 | `SharedMemory` | 内存映射区域 |
| 本地请求/响应套接字 | `LocalSocketListener` 和 `LocalSocketStream` | Unix Socket 或命名管道 |
| Web 前端和本地后端 | `FileChannel` | 带锁的 JSON 文件 |
| 进程内工作线程 | `ThreadChannel` | 线程通道 |
| 事件和进度 | `EventBus` | 进程内事件流 |
| 长时间任务 | `TaskManager` | 任务状态和取消 |
| CLI 进度上报 | `CliBridge` 或 `wrap_command` | 基于 Socket 的任务更新 |

无需安装后台服务。创建通道的进程负责服务端。

## 快速开始

### Python 进程通信

先启动服务端。

~~~python
# server.py
import ipckit

channel = ipckit.IpcChannel.create("demo")
channel.wait_for_client()

request = channel.recv_json()
channel.send_json({"ok": True, "echo": request})
~~~

另一个进程作为客户端。

~~~python
# client.py
import ipckit

channel = ipckit.IpcChannel.connect("demo")
channel.send_json({"message": "hello"})
print(channel.recv_json())
~~~

需要传输原始字节时使用 `NamedPipe`。需要传输 JSON 消息时使用 `IpcChannel`。

### Rust 类型化消息

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

客户端使用 `IpcChannel::<Message>::connect("demo")`，然后调用 `send` 和 `recv`。

## 前后端接入

桌面 WebView 或浏览器前端能够访问共享目录时，可以使用 `FileChannel`。

~~~python
# backend.py
import ipckit

channel = ipckit.FileChannel.backend("./ipc-channel")
request_id = channel.send_request("getData", {"key": "user_info"})
response = channel.wait_response(request_id, timeout_ms=5000)
print(response)
channel.send_event("status", {"ready": True})
~~~

后端写入 `backend_to_frontend.json`，前端写入 `frontend_to_backend.json`。两个文件都是 JSON 消息数组，并通过文件锁保证并发写入安全。

~~~javascript
const response = await fetch("./ipc-channel/backend_to_frontend.json");
const messages = await response.json();
~~~

前端如果由 Python 进程实现，使用 `FileChannel.frontend("./ipc-channel")`。完整的后端、文件前端和 WebView 示例见 [examples/frontend](examples/frontend)。

## 本地套接字

本地套接字 API 在 Unix 和 Windows 上保持一致。

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

JSON 消息使用 `send_json` 和 `recv_json`。自定义字节协议使用 `read_exact` 和 `write_all`。

## 事件和任务

使用 `EventBus` 发布进度或日志事件。

~~~python
import ipckit

bus = ipckit.EventBus()
subscriber = bus.subscribe(ipckit.EventFilter().event_type("task.*"))
publisher = bus.publisher()

publisher.progress("task-1", 1, 2, "running")
event = subscriber.recv_timeout(1000)
print(event.event_type, event.data)
~~~

使用 `TaskManager` 管理可取消任务。

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

## CLI 接入

包装现有命令并获取输出。

~~~python
import ipckit

result = ipckit.wrap_command(
    ["cargo", "build", "--release"],
    task_name="Build",
    task_type="build",
)
print(result.exit_code, result.duration_ms)
~~~

宿主已经提供 ipckit 任务服务时使用 `CliBridge`。

~~~python
import ipckit

with ipckit.CliBridge.connect() as bridge:
    bridge.register_task("Build", "build")
    bridge.set_progress(50, "compiling")
    bridge.complete({"ok": True})
~~~

命令输出中的进度可以使用 `parse_progress(line)` 解析。

## 宿主主线程接入

DCC 和游戏引擎通常要求 UI 操作在主线程执行。Rust 调用方可以从工作线程投递任务，再由宿主的空闲回调执行。

~~~rust
use ipckit::MainThreadPump;
use std::time::Duration;

let pump = MainThreadPump::new();

pump.dispatch(|| {
    // 在这里调用宿主 UI API。
});

// 在宿主的 idle 或 update 回调中调用。
let stats = pump.pump(Duration::from_millis(8));
println!("processed: {}", stats.processed);
~~~

需要记录主线程亲和性时，为任务构造器设置 `ThreadAffinity::Main`。任务实际执行仍由宿主调用 `pump`。

## CLI 命令

~~~bash
ipckit create --type pipe --name demo
ipckit listen --type pipe --name demo
ipckit send --type pipe --name demo "hello"

ipckit monitor --format json
ipckit generate client --type pipe --name demo --output client.rs
~~~

执行 `ipckit --help` 查看全部命令和选项。

## Windows 应用控制策略

如果导入 Windows wheel 时提示应用控制策略阻止了原生 `.pyd`，请检查 CodeIntegrity Operational 日志中的 3077 事件，以及 AppLocker EXE and DLL 日志中的 8004 事件。部署时需要使用策略允许的路径或安装器，为文件添加允许规则，或者使用主机策略信任的证书签名 `.pyd`。

~~~powershell
$pyd = Get-ChildItem "D:\path\to\site-packages\ipckit\*.pyd"
Get-AuthenticodeSignature $pyd.FullName
~~~

移动文件或移除 Mark-of-the-Web 元数据不能替代 App Control 允许规则。

## 构建和测试

~~~bash
python -m pip install maturin pytest pytest-timeout
maturin develop --release
python -m pytest tests
cargo test --workspace
~~~

Python 扩展使用 `python-bindings`、`ext-module` 和 `abi3-py38` 特性构建。

## 项目链接

- [Rust API 文档](https://docs.rs/ipckit)
- [Python 类型声明](python/ipckit/__init__.pyi)
- [示例](examples)
- [变更记录](CHANGELOG.md)

## 许可证

MIT 或 Apache-2.0。
