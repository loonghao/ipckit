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

CLI Bridge (for CLI tool integration):
- CliBridge: Bridge for CLI tools to communicate with frontends
- CliBridgeConfig: Configuration for CLI bridge
- wrap_command(): Wrap a subprocess with CLI bridge integration
- parse_progress(): Parse progress from output lines

Metrics (Issue #10: Performance monitoring):
- ChannelMetrics: Track message counts, latency, throughput
- MetricsSnapshot: Point-in-time snapshot of metrics

API Server (Issue #14: HTTP-over-Socket RESTful API):
- ApiServerConfig: Configuration for API server
- Request: HTTP request object
- Response: HTTP response object
- ApiClient: Client for making API requests

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

    # CLI Bridge usage
    from ipckit import CliBridge, wrap_command

    # Method 1: Using CliBridge directly
    bridge = CliBridge.connect()
    bridge.register_task('My Task', 'custom')
    bridge.set_progress(50, 'Half done')
    bridge.complete({'success': True})

    # Method 2: Wrapping a subprocess
    output = wrap_command(['pip', 'install', 'requests'], task_name='Install')
    print(f'Exit code: {output.exit_code}')

    # Metrics usage
    from ipckit import ChannelMetrics

    metrics = ChannelMetrics()
    metrics.record_send(100)
    print(f'Messages sent: {metrics.messages_sent}')
    print(metrics.to_prometheus('ipckit'))

    # API Client usage
    from ipckit import ApiClient

    client = ApiClient.connect()
    tasks = client.get('/v1/tasks')
"""

from .ipckit import (
    AnonymousPipe,
    ApiClient,
    ApiServerConfig,
    ChannelMetrics,
    CliBridge,
    CliBridgeConfig,
    CommandOutput,
    FileChannel,
    GracefulIpcChannel,
    GracefulNamedPipe,
    IpcChannel,
    MetricsSnapshot,
    NamedPipe,
    ProgressInfo,
    Request,
    Response,
    SharedMemory,
    __version__,
    json_dumps,
    json_dumps_pretty,
    json_loads,
    parse_progress,
    wrap_command,
)

__all__ = [
    # Core IPC
    "AnonymousPipe",
    "NamedPipe",
    "SharedMemory",
    "IpcChannel",
    "FileChannel",
    # Graceful shutdown
    "GracefulNamedPipe",
    "GracefulIpcChannel",
    # CLI Bridge
    "CliBridge",
    "CliBridgeConfig",
    "ProgressInfo",
    "CommandOutput",
    "wrap_command",
    "parse_progress",
    # Metrics (Issue #10)
    "ChannelMetrics",
    "MetricsSnapshot",
    # API Server (Issue #14)
    "ApiServerConfig",
    "Request",
    "Response",
    "ApiClient",
    # JSON utilities
    "json_dumps",
    "json_dumps_pretty",
    "json_loads",
    # Version
    "__version__",
]
