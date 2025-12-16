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
