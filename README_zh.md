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

## 📖 IPC 方式对比

| 方式 | 使用场景 | 性能 | 复杂度 |
|------|----------|------|--------|
| **匿名管道** | 父子进程 | 快速 | 低 |
| **命名管道** | 无关进程 | 快速 | 中等 |
| **共享内存** | 大数据、频繁访问 | 最快 | 高 |
| **IPC 通道** | 消息传递 | 快速 | 低 |
| **文件通道** | 前后端通信 | 中等 | 低 |

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
