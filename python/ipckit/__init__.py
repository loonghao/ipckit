"""
ipckit - A cross-platform IPC (Inter-Process Communication) library

This library provides various IPC mechanisms:
- AnonymousPipe: For parent-child process communication
- NamedPipe: For communication between unrelated processes
- SharedMemory: For fast data sharing between processes
- IpcChannel: High-level message passing interface
- FileChannel: File-based IPC for frontend-backend communication

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
    
    # File-based IPC (for frontend-backend)
    from ipckit import FileChannel
    
    # Backend
    channel = FileChannel.backend('./ipc_channel')
    channel.send_request('ping', {})
    
    # Fast JSON (Rust-native)
    from ipckit import json_dumps, json_loads
    
    json_str = json_dumps({'key': 'value'})
    data = json_loads(json_str)
"""

from .ipckit import (
    AnonymousPipe,
    NamedPipe,
    SharedMemory,
    IpcChannel,
    FileChannel,
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
    "json_dumps",
    "json_dumps_pretty",
    "json_loads",
    "__version__",
]
