"""
ipckit - A cross-platform IPC (Inter-Process Communication) library

This library provides various IPC mechanisms:
- AnonymousPipe: For parent-child process communication
- NamedPipe: For communication between unrelated processes
- SharedMemory: For fast data sharing between processes
- IpcChannel: High-level message passing interface
- FileChannel: File-based IPC for frontend-backend communication

Graceful shutdown support:
- GracefulNamedPipe: Named pipe with graceful shutdown
- GracefulIpcChannel: IPC channel with graceful shutdown

JSON utilities (faster than Python's json module, powered by Rust serde_json):
- json_dumps(obj): Serialize Python object to JSON string
- json_dumps_pretty(obj): Serialize with pretty formatting
- json_loads(s): Deserialize JSON string to Python object

Example:
    # Server
    from ipckit import IpcChannel

    channel = IpcChannel.create('my_channel')
    channel.wait_for_client()
    data = channel.recv()
    print(f"Received: {data}")

    # Client (in another process)
    from ipckit import IpcChannel

    channel = IpcChannel.connect('my_channel')
    channel.send(b'Hello, IPC!')
    
    # Using graceful shutdown
    from ipckit import GracefulIpcChannel
    
    channel = GracefulIpcChannel.create('my_channel')
    channel.wait_for_client()
    
    # ... use channel ...
    
    # Graceful shutdown
    channel.shutdown()
    channel.drain()  # Wait for pending operations
    
    # Or with timeout (in milliseconds)
    channel.shutdown_timeout(5000)
"""

from .ipckit import (
    AnonymousPipe,
    NamedPipe,
    SharedMemory,
    IpcChannel,
    FileChannel,
    GracefulNamedPipe,
    GracefulIpcChannel,
    json_dumps,
    json_dumps_pretty,
    json_loads,
    __version__,
)

__all__ = [
    "AnonymousPipe",
    "NamedPipe",
    "SharedMemory",
    "IpcChannel",
    "FileChannel",
    "GracefulNamedPipe",
    "GracefulIpcChannel",
    "json_dumps",
    "json_dumps_pretty",
    "json_loads",
    "__version__",
]
