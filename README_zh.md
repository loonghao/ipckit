# ipckit

[![Crates.io](https://img.shields.io/crates/v/ipckit.svg)](https://crates.io/crates/ipckit)
[![PyPI](https://img.shields.io/pypi/v/ipckit.svg)](https://pypi.org/project/ipckit/)
[![Documentation](https://docs.rs/ipckit/badge.svg)](https://docs.rs/ipckit)
[![CI](https://github.com/loonghao/ipckit/actions/workflows/ci.yml/badge.svg)](https://github.com/loonghao/ipckit/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Python Versions](https://img.shields.io/pypi/pyversions/ipckit.svg)](https://pypi.org/project/ipckit/)
[![Rust Version](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Downloads](https://img.shields.io/pypi/dm/ipckit.svg)](https://pypi.org/project/ipckit/)

一个高性能、跨平台的进程间通信 (IPC) 库，基于 Rust 构建，同时支持 Rust 和 Python。

[English](README.md)

## ✨ 特性

- 🚀 **高性能** - 使用 Rust 编写，尽可能实现零拷贝
- 🔀 **跨平台** - 支持 Windows、Linux 和 macOS
- 🐍 **Python 绑定** - 通过 PyO3 提供一流的 Python 支持
- 📦 **多种 IPC 方式** - 管道、共享内存、通道和基于文件的 IPC
- 🔒 **线程安全** - 跨进程安全并发访问
- ⚡ **原生 JSON** - 使用 Rust 的 serde_json 内置快速 JSON 序列化
- 🛡️ **优雅关闭** - 内置优雅关闭通道支持
- 🔌 **本地套接字** - Unix Domain Socket / Named Pipe 抽象，实现跨平台套接字通信
- 🧵 **线程通道** - 高性能进程内线程通信
- 📡 **事件流** - 实时发布-订阅事件系统
- 📋 **任务管理器** - 带进度跟踪的任务生命周期管理
- 🌐 **Socket 服务器** - 多客户端 Socket 服务器（类似 Docker 的 socket）
- 🔧 **CLI 桥接** - 将 CLI 工具与实时进度和通信集成
- 📊 **通道指标** - 内置发送/接收操作的指标跟踪
- 🛠️ **CLI 工具** - 代码生成和通道监控命令
- 📝 **声明式宏** - 便捷的通道创建和命令路由宏

## 📦 安装

### Python

```bash
pip install ipckit
```

### Rust

```toml
[dependencies]
ipckit = "0.1"
```

## 🚀 快速开始

### 匿名管道（父子进程通信）

**Python:**
```python
import ipckit
import subprocess

# 创建管道对
pipe = ipckit.AnonymousPipe()

# 写入管道
pipe.write(b"来自父进程的消息！")

# 从管道读取
data = pipe.read(1024)
print(data)
```

**Rust:**
```rust
use ipckit::AnonymousPipe;

fn main() -> ipckit::Result<()> {
    let pipe = AnonymousPipe::new()?;
    
    pipe.write_all(b"来自 Rust 的消息！")?;
    
    let mut buf = [0u8; 1024];
    let n = pipe.read(&mut buf)?;
    println!("{}", String::from_utf8_lossy(&buf[..n]));
    
    Ok(())
}
```

### 命名管道（无关进程通信）

**Python 服务端:**
```python
import ipckit

# 创建服务端
server = ipckit.NamedPipe.create("my_pipe")
print("等待客户端连接...")
server.wait_for_client()

# 通信
data = server.read(1024)
server.write(b"来自服务端的响应")
```

**Python 客户端:**
```python
import ipckit

# 连接到服务端
client = ipckit.NamedPipe.connect("my_pipe")

# 通信
client.write(b"来自客户端的消息")
response = client.read(1024)
print(response)
```

### 共享内存（快速数据交换）

**Python:**
```python
import ipckit

# 创建共享内存（所有者）
shm = ipckit.SharedMemory.create("my_shm", 4096)
shm.write(0, b"共享的数据！")

# 在另一个进程中打开
shm2 = ipckit.SharedMemory.open("my_shm")
data = shm2.read(0, 15)
print(data)  # b"共享的数据！"
```

**Rust:**
```rust
use ipckit::SharedMemory;

fn main() -> ipckit::Result<()> {
    // 创建
    let shm = SharedMemory::create("my_shm", 4096)?;
    shm.write(0, b"来自 Rust 的数据！")?;
    
    // 在另一个进程中打开
    let shm2 = SharedMemory::open("my_shm")?;
    let data = shm2.read(0, 20)?;
    
    Ok(())
}
```

### IPC 通道（高级消息传递）

**Python:**
```python
import ipckit

# 服务端
channel = ipckit.IpcChannel.create("my_channel")
channel.wait_for_client()

# 发送/接收 JSON
channel.send_json({"type": "greeting", "message": "你好！"})
response = channel.recv_json()
print(response)
```

### 文件通道（前后端通信）

非常适合桌面应用程序，Python 后端与 Web 前端通信。

**Python 后端:**
```python
import ipckit

# 创建后端通道
channel = ipckit.FileChannel.backend("./ipc_channel")

# 向前端发送请求
request_id = channel.send_request("getData", {"key": "user_info"})

# 等待响应
response = channel.wait_response(request_id, timeout_ms=5000)
print(response)

# 发送事件
channel.send_event("status_update", {"status": "ready"})
```

**JavaScript 前端:**
```javascript
// 读取: ./ipc_channel/backend_to_frontend.json
// 写入: ./ipc_channel/frontend_to_backend.json

async function pollMessages() {
    const response = await fetch('./ipc_channel/backend_to_frontend.json');
    const messages = await response.json();
    // 处理新消息...
}
```

### 原生 JSON 函数

ipckit 提供 Rust 原生的 JSON 函数，比 Python 内置的 json 模块更快：

```python
import ipckit

# 序列化（比 json.dumps 快 1.2 倍）
data = {"name": "test", "values": [1, 2, 3]}
json_str = ipckit.json_dumps(data)

# 美化输出
pretty_str = ipckit.json_dumps_pretty(data)

# 反序列化
obj = ipckit.json_loads('{"key": "value"}')
```

### 优雅关闭

当使用 IPC 通道与事件循环（如 WebView、GUI 框架）配合时，后台线程可能在主事件循环关闭后继续发送消息，导致错误。`GracefulChannel` 功能解决了这个问题。

**Python:**
```python
import ipckit

# 创建支持优雅关闭的通道
channel = ipckit.GracefulIpcChannel.create("my_channel")
channel.wait_for_client()

# ... 正常使用通道 ...
data = channel.recv()
channel.send(b"response")

# 优雅关闭 - 阻止新操作并等待待处理操作完成
channel.shutdown()
channel.drain()  # 等待所有待处理操作完成

# 或者使用带超时的关闭（毫秒）
channel.shutdown_timeout(5000)  # 5 秒超时
```

**主要优势:**
- 防止 `EventLoopClosed` 等类似错误
- 线程安全的关闭信号
- 使用 RAII 守卫跟踪待处理操作
- 可配置的排空超时

### 本地套接字（跨平台套接字通信）

本地套接字为 Unix Domain Sockets（Unix/macOS）和 Named Pipes（Windows）提供统一的 API。

**Python 服务端:**
```python
import ipckit

# 创建服务端
server = ipckit.LocalSocketListener.bind("my_socket")
print("等待客户端连接...")

# 接受连接
stream = server.accept()

# 接收和发送数据
data = stream.read(1024)
print(f"收到: {data}")
stream.write(b"来自服务端的消息！")

# JSON 通信
json_data = stream.recv_json()
stream.send_json({"status": "ok", "message": "已收到"})
```

**Python 客户端:**
```python
import ipckit

# 连接到服务端
stream = ipckit.LocalSocketStream.connect("my_socket")

# 发送和接收数据
stream.write(b"来自客户端的消息！")
response = stream.read(1024)
print(f"响应: {response}")

# JSON 通信
stream.send_json({"action": "getData", "key": "user"})
result = stream.recv_json()
print(result)
```

**主要优势:**
- 跨平台：支持 Windows、Linux 和 macOS
- 双向通信
- 内置带长度前缀的 JSON 序列化
- 简单的客户端-服务端模型

### 事件流（发布-订阅）

实时事件系统，用于任务进度、日志和通知。

**Python:**
```python
import ipckit

# 创建事件总线
bus = ipckit.EventBus()
publisher = bus.publisher()

# 订阅任务事件
subscriber = bus.subscribe(ipckit.EventFilter().event_type("task.*"))

# 发布事件
publisher.progress("task-123", 50, 100, "完成一半")
publisher.log("task-123", "info", "处理中...")

# 接收事件（非阻塞）
while event := subscriber.try_recv():
    print(f"[{event.event_type}] {event.data}")

# 或者带超时
try:
    event = subscriber.recv_timeout(1000)  # 1 秒
except RuntimeError:
    print("超时")
```

**Rust:**
```rust
use ipckit::{EventBus, Event, EventFilter};

fn main() {
    let bus = EventBus::new(Default::default());
    let publisher = bus.publisher();

    // 订阅任务事件
    let subscriber = bus.subscribe(
        EventFilter::new().event_type("task.*")
    );

    // 发布事件
    publisher.progress("task-123", 50, 100, "完成一半");
    publisher.log("task-123", "info", "处理中...");

    // 接收事件
    while let Some(event) = subscriber.try_recv() {
        println!("[{}] {:?}", event.event_type, event.data);
    }
}
```

### 任务管理器（任务生命周期）

管理长时间运行的任务，支持进度跟踪和取消。

**Python:**
```python
import ipckit
import time

manager = ipckit.TaskManager()

# 创建任务
handle = manager.create_task("上传文件", "upload")
handle.start()

# 模拟工作
for i in range(100):
    if handle.is_cancelled:
        handle.fail("用户取消")
        break
    handle.set_progress(i + 1, f"步骤 {i + 1}/100")
    time.sleep(0.01)
else:
    handle.complete({"uploaded": 100})

# 列出活动任务
active = manager.list_active()
print(f"活动任务: {len(active)}")

# 取消任务
# manager.cancel(handle.id)
```

**Rust:**
```rust
use ipckit::{TaskManager, TaskBuilder, TaskFilter};
use std::time::Duration;

fn main() {
    let manager = TaskManager::new(Default::default());

    // 启动任务
    let handle = manager.spawn("上传文件", "upload", |task| {
        for i in 0..100 {
            if task.is_cancelled() {
                return;
            }
            task.set_progress(i + 1, Some(&format!("步骤 {}/100", i + 1)));
            std::thread::sleep(Duration::from_millis(50));
        }
        task.complete(serde_json::json!({"uploaded": 100}));
    });

    // 列出活动任务
    let active = manager.list(&TaskFilter::new().active());
    println!("活动任务: {}", active.len());

    // 如需取消
    // manager.cancel(handle.id()).unwrap();
}
```

### CLI 桥接（CLI 工具集成）

将任何 CLI 工具与实时进度跟踪和双向通信集成。

**Python:**
```python
import ipckit

# 方法 1：直接使用 CliBridge
bridge = ipckit.CliBridge()
bridge.register_task("构建项目", "build")

for i in range(100):
    if bridge.is_cancelled:
        bridge.fail("用户取消")
        break
    bridge.set_progress(i + 1, f"步骤 {i + 1}/100")

bridge.complete({"success": True})

# 方法 2：包装现有命令并解析进度
output = ipckit.wrap_command(
    ["cargo", "build", "--release"],
    task_name="构建项目",
    task_type="build"
)
print(f"退出码: {output.exit_code}")
print(f"耗时: {output.duration_ms}ms")

# 方法 3：从输出解析进度
info = ipckit.parse_progress("下载中... 75%", "percentage")
print(f"进度: {info.percentage}%")
```

**Rust:**
```rust
use ipckit::{CliBridge, WrappedCommand, parsers};

fn main() -> ipckit::Result<()> {
    // 方法 1：直接使用桥接
    let bridge = CliBridge::connect()?;
    bridge.register_task("我的任务", "build")?;
    
    for i in 0..100 {
        if bridge.is_cancelled() {
            bridge.fail("已取消");
            return Ok(());
        }
        bridge.set_progress(i + 1, Some(&format!("步骤 {}/100", i + 1)));
    }
    bridge.complete(serde_json::json!({"success": true}));

    // 方法 2：包装现有命令
    let output = WrappedCommand::new("cargo")
        .args(["build", "--release"])
        .task("构建项目", "build")
        .progress_parser(parsers::PercentageParser)
        .run()?;
    
    println!("退出码: {}", output.exit_code);
    Ok(())
}
```

**主要功能:**
- 自动捕获和转发 stdout/stderr
- 内置进度解析器（百分比、分数、进度条）
- 任务取消支持
- 最小侵入性 - 现有 CLI 只需最少修改

### API 服务器（基于本地套接字的 HTTP 风格 API）

对于 Python 服务端应用，我们推荐集成流行的异步框架如 [FastAPI](https://fastapi.tiangolo.com/) 或 [Robyn](https://robyn.tech/)。这些框架提供了健壮的路由、中间件和异步支持。

**Python 使用 FastAPI + Uvicorn（Unix Socket）：**
```python
# server.py
from fastapi import FastAPI
import uvicorn

app = FastAPI()

@app.get("/v1/health")
async def health():
    return {"status": "ok"}

@app.post("/v1/tasks")
async def create_task(data: dict):
    return {"id": "task-123", "name": data.get("name")}

# 在 Unix socket 上运行
if __name__ == "__main__":
    uvicorn.run(app, uds="/tmp/my_api.sock")
```

**Python 使用 Robyn（高性能）：**
```python
# server.py
from robyn import Robyn

app = Robyn(__file__)

@app.get("/v1/health")
async def health():
    return {"status": "ok"}

@app.post("/v1/tasks")
async def create_task(request):
    data = request.json()
    return {"id": "task-123", "name": data.get("name")}

# Robyn 通过配置支持 Unix sockets
app.start(host="0.0.0.0", port=8080)
```

**Python 客户端（使用 ipckit）：**
```python
import ipckit

# 连接到 API 服务器
client = ipckit.ApiClient("/tmp/my_api.sock")

# 发送请求
health = client.get("/v1/health")
print(health)  # {"status": "ok"}

task = client.post("/v1/tasks", {"name": "my-task"})
print(task)  # {"id": "task-123", "name": "my-task"}
```

**Rust 服务端：**
```rust
use ipckit::{ApiServer, ApiServerConfig, Router, Response};

fn main() -> ipckit::Result<()> {
    let config = ApiServerConfig::new("/tmp/my_api.sock");
    
    let router = Router::new()
        .get("/v1/health", |_req| {
            Response::ok(serde_json::json!({"status": "ok"}))
        })
        .post("/v1/tasks", |req| {
            let data = req.json::<serde_json::Value>()?;
            Response::created(serde_json::json!({
                "id": "task-123",
                "name": data.get("name")
            }))
        });
    
    let server = ApiServer::new(config, router)?;
    server.run()?;
    Ok(())
}
```

### 通道指标（性能监控）

使用内置指标跟踪发送/接收操作。

**Rust:**
```rust
use ipckit::{ChannelMetrics, MeteredSender, MeteredReceiver, metered_pair, AggregatedMetrics};
use std::sync::Arc;

fn main() {
    // 创建带指标的发送/接收对
    let (tx, rx) = metered_pair(crossbeam_channel::unbounded());
    
    // 发送消息
    tx.send("Hello".to_string()).unwrap();
    tx.send("World".to_string()).unwrap();
    
    // 接收消息
    let _ = rx.recv().unwrap();
    
    // 获取指标
    let metrics = tx.metrics();
    println!("已发送: {}, 已接收: {}", metrics.messages_sent(), metrics.messages_received());
    
    // 聚合多个通道的指标
    let mut aggregated = AggregatedMetrics::new();
    aggregated.add_channel("channel1", metrics.clone());
    
    // 导出为 JSON 或 Prometheus 格式
    println!("{}", aggregated.to_json());
    println!("{}", aggregated.to_prometheus());
}
```

### CLI 工具

ipckit 提供代码生成和通道监控的 CLI 工具。

**代码生成:**
```bash
# 生成客户端代码
ipckit generate client --name MyClient --output ./src/client.rs

# 生成服务端代码
ipckit generate server --name MyServer --output ./src/server.rs

# 生成 Python 绑定
ipckit generate python --name my_module --output ./bindings/

# 生成消息处理器
ipckit generate handler --name MessageHandler --output ./src/handler.rs
```

**通道监控:**
```bash
# 使用 TUI 界面监控通道
ipckit monitor --channel my_channel

# 使用 JSON 格式输出
ipckit monitor --channel my_channel --format json

# 自定义刷新间隔
ipckit monitor --channel my_channel --interval 500
```

### 声明式宏

用于常见 IPC 模式的便捷宏。

**Rust:**
```rust
use ipckit::{ipc_channel, ipc_commands, ipc_message, ipc_middleware};

fn main() {
    // 使用单个宏创建通道
    let (tx, rx) = ipc_channel!(String, "my_channel");
    
    // 定义消息类型
    ipc_message! {
        struct UserRequest {
            user_id: u64,
            action: String,
        }
    }
    
    // 定义命令路由
    ipc_commands! {
        "ping" => handle_ping,
        "echo" => handle_echo,
        "status" => handle_status,
    }
    
    // 链式中间件
    ipc_middleware! {
        logging_middleware,
        auth_middleware,
        => final_handler
    }
}

fn handle_ping() -> String { "pong".to_string() }
fn handle_echo() -> String { "echo".to_string() }
fn handle_status() -> String { "ok".to_string() }
fn logging_middleware<F: Fn() -> String>(next: F) -> String { next() }
fn auth_middleware<F: Fn() -> String>(next: F) -> String { next() }
fn final_handler() -> String { "done".to_string() }
```

## 📖 IPC 方式对比

| 方式 | 使用场景 | 性能 | 复杂度 |
|------|----------|------|--------|
| **匿名管道** | 父子进程 | 快速 | 低 |
| **命名管道** | 无关进程 | 快速 | 中等 |
| **共享内存** | 大数据、频繁访问 | 最快 | 高 |
| **IPC 通道** | 消息传递 | 快速 | 低 |
| **文件通道** | 前后端通信 | 中等 | 低 |
| **优雅通道** | 事件循环集成 | 快速 | 低 |
| **本地套接字** | 跨平台套接字 | 快速 | 低 |
| **线程通道** | 进程内线程 | 最快 | 低 |
| **事件流** | 发布-订阅事件 | 快速 | 低 |
| **任务管理器** | 任务生命周期 | 快速 | 中等 |
| **Socket 服务器** | 多客户端服务器 | 快速 | 中等 |
| **CLI 桥接** | CLI 工具集成 | 快速 | 低 |
| **通道指标** | 性能监控 | 快速 | 低 |
| **CLI 工具** | 代码生成和监控 | N/A | 低 |
| **声明式宏** | 减少样板代码 | N/A | 低 |

## 🏗️ 架构

```
┌─────────────────────────────────────────────────────────────┐
│                     Python 应用程序                          │
├─────────────────────────────────────────────────────────────┤
│                    ipckit Python 绑定                        │
│                         (PyO3)                               │
├─────────────────────────────────────────────────────────────┤
│                     ipckit Rust 核心                         │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────────────────┐│
│  │   管道  │ │ 共享内存│ │   通道  │ │      文件通道       ││
│  └─────────┘ └─────────┘ └─────────┘ └─────────────────────┘│
│  ┌─────────────────────────────────────────────────────────┐│
│  │                   优雅关闭层                            ││
│  │  (GracefulNamedPipe, GracefulIpcChannel, ShutdownState) ││
│  └─────────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────────┐│
│  │                   本地套接字层                          ││
│  │       (LocalSocketListener, LocalSocketStream)          ││
│  └─────────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────────┐│
│  │                   高级服务层                            ││
│  │  (ThreadChannel, EventStream, TaskManager, SocketServer)││
│  │  (CliBridge, WrappedCommand)                            ││
│  └─────────────────────────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────┤
│                     平台抽象层                               │
│              (Windows / Linux / macOS)                       │
└─────────────────────────────────────────────────────────────┘
```

## 🔧 从源码构建

### 前置条件

- Rust 1.70+
- Python 3.7+
- maturin (`pip install maturin`)

### 构建

```bash
# 克隆仓库
git clone https://github.com/loonghao/ipckit.git
cd ipckit

# 构建 Python 包
maturin develop --release

# 运行测试
pytest tests/
cargo test
```

## 📝 许可证

本项目采用双重许可：

- [MIT 许可证](LICENSE-MIT)
- [Apache 许可证 2.0](LICENSE-APACHE)

## 🤝 贡献

欢迎贡献！请随时提交 Pull Request。

1. Fork 本仓库
2. 创建你的特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交你的更改 (`git commit -m '添加一些很棒的特性'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 打开一个 Pull Request

## 📚 文档

- [API 文档 (Rust)](https://docs.rs/ipckit)
- [API 文档 (Python)](https://github.com/loonghao/ipckit/wiki)
- [示例](examples/)

## 🙏 致谢

- [PyO3](https://pyo3.rs/) - Python 的 Rust 绑定
- [maturin](https://www.maturin.rs/) - 构建和发布基于 Rust 的 Python 包
- [serde](https://serde.rs/) - Rust 的序列化框架
