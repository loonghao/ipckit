"""Type stubs for ipckit"""

from typing import Any

__version__: str

# JSON utilities (Rust-native, faster than Python's json module)

def json_dumps(obj: Any) -> str:
    """Serialize Python object to JSON string using Rust serde_json.

    This is faster than Python's json.dumps() for most use cases.

    Args:
        obj: Python object to serialize (dict, list, str, int, float, bool, None)

    Returns:
        JSON string

    Raises:
        ValueError: If object cannot be serialized to JSON
        TypeError: If object type is not supported
    """
    ...

def json_dumps_pretty(obj: Any) -> str:
    """Serialize Python object to pretty-formatted JSON string.

    Args:
        obj: Python object to serialize

    Returns:
        Pretty-formatted JSON string with indentation
    """
    ...

def json_loads(s: str) -> Any:
    """Deserialize JSON string to Python object using Rust serde_json.

    Args:
        s: JSON string to parse

    Returns:
        Python object (dict, list, str, int, float, bool, or None)

    Raises:
        ValueError: If string is not valid JSON
    """
    ...

class AnonymousPipe:
    """Anonymous pipe for parent-child process communication."""

    def __init__(self) -> None:
        """Create a new anonymous pipe pair."""
        ...

    def read(self, size: int) -> bytes:
        """Read data from the pipe.

        Args:
            size: Maximum number of bytes to read.

        Returns:
            Data read from the pipe.
        """
        ...

    def write(self, data: bytes) -> int:
        """Write data to the pipe.

        Args:
            data: Data to write.

        Returns:
            Number of bytes written.
        """
        ...

    def reader_fd(self) -> int:
        """Get the reader file descriptor (Unix only)."""
        ...

    def writer_fd(self) -> int:
        """Get the writer file descriptor (Unix only)."""
        ...

    def take_reader(self) -> None:
        """Take the reader end (for passing to child process)."""
        ...

    def take_writer(self) -> None:
        """Take the writer end (for passing to child process)."""
        ...

class NamedPipe:
    """Named pipe for communication between unrelated processes."""

    @staticmethod
    def create(name: str) -> NamedPipe:
        """Create a new named pipe server.

        Args:
            name: Pipe name.

        Returns:
            A new NamedPipe instance.
        """
        ...

    @staticmethod
    def connect(name: str) -> NamedPipe:
        """Connect to an existing named pipe.

        Args:
            name: Pipe name to connect to.

        Returns:
            A connected NamedPipe instance.
        """
        ...

    @property
    def name(self) -> str:
        """Get the pipe name."""
        ...

    @property
    def is_server(self) -> bool:
        """Check if this is the server end."""
        ...

    def wait_for_client(self) -> None:
        """Wait for a client to connect (server only)."""
        ...

    def read(self, size: int) -> bytes:
        """Read data from the pipe."""
        ...

    def write(self, data: bytes) -> int:
        """Write data to the pipe."""
        ...

    def read_exact(self, size: int) -> bytes:
        """Read exact number of bytes."""
        ...

    def write_all(self, data: bytes) -> None:
        """Write all data."""
        ...

class SharedMemory:
    """Shared memory region for fast data exchange between processes."""

    @staticmethod
    def create(name: str, size: int) -> SharedMemory:
        """Create a new shared memory region.

        Args:
            name: Unique name for the shared memory.
            size: Size in bytes.

        Returns:
            A new SharedMemory instance.
        """
        ...

    @staticmethod
    def open(name: str) -> SharedMemory:
        """Open an existing shared memory region.

        Args:
            name: Name of the shared memory to open.

        Returns:
            A SharedMemory instance.
        """
        ...

    @property
    def name(self) -> str:
        """Get the shared memory name."""
        ...

    @property
    def size(self) -> int:
        """Get the shared memory size."""
        ...

    @property
    def is_owner(self) -> bool:
        """Check if this instance is the owner."""
        ...

    def write(self, offset: int, data: bytes) -> None:
        """Write data to shared memory at offset.

        Args:
            offset: Byte offset to write at.
            data: Data to write.
        """
        ...

    def read(self, offset: int, size: int) -> bytes:
        """Read data from shared memory at offset.

        Args:
            offset: Byte offset to read from.
            size: Number of bytes to read.

        Returns:
            Data read from shared memory.
        """
        ...

    def read_all(self) -> bytes:
        """Read all data from shared memory."""
        ...

class IpcChannel:
    """High-level IPC channel for message passing."""

    @staticmethod
    def create(name: str) -> IpcChannel:
        """Create a new IPC channel server.

        Args:
            name: Channel name.

        Returns:
            A new IpcChannel instance.
        """
        ...

    @staticmethod
    def connect(name: str) -> IpcChannel:
        """Connect to an existing IPC channel.

        Args:
            name: Channel name to connect to.

        Returns:
            A connected IpcChannel instance.
        """
        ...

    @property
    def name(self) -> str:
        """Get the channel name."""
        ...

    @property
    def is_server(self) -> bool:
        """Check if this is the server end."""
        ...

    def wait_for_client(self) -> None:
        """Wait for a client to connect (server only)."""
        ...

    def send(self, data: bytes) -> None:
        """Send bytes through the channel.

        Args:
            data: Data to send.
        """
        ...

    def recv(self) -> bytes:
        """Receive bytes from the channel.

        Returns:
            Received data.
        """
        ...

    def send_json(self, obj: Any) -> None:
        """Send a JSON-serializable object.

        Args:
            obj: Object to send (will be serialized to JSON).
        """
        ...

    def recv_json(self) -> Any:
        """Receive a JSON object.

        Returns:
            Deserialized Python object.
        """
        ...

class FileChannel:
    """File-based IPC channel for frontend-backend communication.

    This provides a simple file-based IPC mechanism where:
    - Backend writes to one file, Frontend reads it
    - Frontend writes to another file, Backend reads it

    All JSON serialization is done in Rust for better performance.

    Example:
        # Backend (Python)
        channel = FileChannel.backend('./ipc_channel')
        request_id = channel.send_request('ping', {})
        response = channel.wait_response(request_id, timeout_ms=5000)

        # Frontend reads: ./ipc_channel/backend_to_frontend.json
        # Frontend writes: ./ipc_channel/frontend_to_backend.json
    """

    @staticmethod
    def backend(dir: str) -> FileChannel:
        """Create a backend-side file channel.

        Args:
            dir: Directory for channel files (will be created if not exists)

        Returns:
            A new FileChannel instance for backend use.
        """
        ...

    @staticmethod
    def frontend(dir: str) -> FileChannel:
        """Create a frontend-side file channel.

        Args:
            dir: Directory for channel files

        Returns:
            A new FileChannel instance for frontend use.
        """
        ...

    @property
    def dir(self) -> str:
        """Get the channel directory path."""
        ...

    def send_request(self, method: str, params: Any) -> str:
        """Send a request message.

        Args:
            method: Method name to call
            params: Parameters as a dict (will be serialized to JSON)

        Returns:
            The request ID (use this to match the response)
        """
        ...

    def send_response(self, request_id: str, result: Any) -> None:
        """Send a response to a request.

        Args:
            request_id: The ID of the request being responded to
            result: The result data (will be serialized to JSON)
        """
        ...

    def send_error(self, request_id: str, error: str) -> None:
        """Send an error response.

        Args:
            request_id: The ID of the request being responded to
            error: Error message
        """
        ...

    def send_event(self, name: str, payload: Any) -> None:
        """Send an event (fire-and-forget, no response expected).

        Args:
            name: Event name
            payload: Event data (will be serialized to JSON)
        """
        ...

    def recv(self) -> list[dict[str, Any]]:
        """Receive all new messages.

        Returns:
            List of message dicts, each containing:
            - id: Message ID
            - timestamp: Unix timestamp in milliseconds
            - type: "request", "response", or "event"
            - method: Method name (for requests/events)
            - payload: Message data
            - reply_to: Request ID (for responses)
            - error: Error message (for error responses)
        """
        ...

    def recv_one(self) -> dict[str, Any] | None:
        """Receive a single new message (non-blocking).

        Returns:
            Message dict if available, None otherwise
        """
        ...

    def wait_response(self, request_id: str, timeout_ms: int) -> dict[str, Any]:
        """Wait for a response to a specific request.

        Args:
            request_id: The ID of the request to wait for
            timeout_ms: Timeout in milliseconds

        Returns:
            Response message dict

        Raises:
            TimeoutError: If no response received within timeout
        """
        ...

    def clear(self) -> None:
        """Clear all messages in both inbox and outbox."""
        ...

class GracefulNamedPipe:
    """Named pipe with graceful shutdown support.

    This class wraps a NamedPipe with graceful shutdown capabilities,
    preventing errors when background threads continue sending messages
    after the main event loop has closed.

    Example:
        channel = GracefulNamedPipe.create('my_pipe')
        channel.wait_for_client()

        # ... use channel ...

        # Graceful shutdown
        channel.shutdown()
        channel.drain()  # Wait for pending operations

        # Or with timeout (in milliseconds)
        channel.shutdown_timeout(5000)
    """

    @staticmethod
    def create(name: str) -> GracefulNamedPipe:
        """Create a new named pipe server with graceful shutdown.

        Args:
            name: Pipe name.

        Returns:
            A new GracefulNamedPipe instance.
        """
        ...

    @staticmethod
    def connect(name: str) -> GracefulNamedPipe:
        """Connect to an existing named pipe with graceful shutdown.

        Args:
            name: Pipe name to connect to.

        Returns:
            A connected GracefulNamedPipe instance.
        """
        ...

    @property
    def name(self) -> str:
        """Get the pipe name."""
        ...

    @property
    def is_server(self) -> bool:
        """Check if this is the server end."""
        ...

    @property
    def is_shutdown(self) -> bool:
        """Check if the channel has been shutdown."""
        ...

    def wait_for_client(self) -> None:
        """Wait for a client to connect (server only).

        Raises:
            ConnectionError: If channel is already shutdown
        """
        ...

    def shutdown(self) -> None:
        """Signal the channel to shutdown.

        After calling this method:
        - New send/receive operations will raise ConnectionError
        - Pending operations may still complete
        - Use drain() to wait for pending operations
        """
        ...

    def drain(self) -> None:
        """Wait for all pending operations to complete."""
        ...

    def shutdown_timeout(self, timeout_ms: int) -> None:
        """Shutdown with a timeout.

        Combines shutdown() and drain() with a timeout.

        Args:
            timeout_ms: Timeout in milliseconds

        Raises:
            TimeoutError: If drain doesn't complete within timeout
        """
        ...

    def read(self, size: int) -> bytes:
        """Read data from the pipe.

        Raises:
            BrokenPipeError: If channel is shutdown
        """
        ...

    def write(self, data: bytes) -> int:
        """Write data to the pipe.

        Raises:
            BrokenPipeError: If channel is shutdown
        """
        ...

    def read_exact(self, size: int) -> bytes:
        """Read exact number of bytes.

        Raises:
            BrokenPipeError: If channel is shutdown
        """
        ...

    def write_all(self, data: bytes) -> None:
        """Write all data.

        Raises:
            BrokenPipeError: If channel is shutdown
        """
        ...

class GracefulIpcChannel:
    """IPC channel with graceful shutdown support.

    This class wraps an IpcChannel with graceful shutdown capabilities,
    preventing errors when background threads continue sending messages
    after the main event loop has closed.

    Example:
        channel = GracefulIpcChannel.create('my_channel')
        channel.wait_for_client()

        # ... use channel ...

        # Graceful shutdown
        channel.shutdown()
        channel.drain()  # Wait for pending operations

        # Or with timeout (in milliseconds)
        channel.shutdown_timeout(5000)
    """

    @staticmethod
    def create(name: str) -> GracefulIpcChannel:
        """Create a new IPC channel server with graceful shutdown.

        Args:
            name: Channel name.

        Returns:
            A new GracefulIpcChannel instance.
        """
        ...

    @staticmethod
    def connect(name: str) -> GracefulIpcChannel:
        """Connect to an existing IPC channel with graceful shutdown.

        Args:
            name: Channel name to connect to.

        Returns:
            A connected GracefulIpcChannel instance.
        """
        ...

    @property
    def name(self) -> str:
        """Get the channel name."""
        ...

    @property
    def is_server(self) -> bool:
        """Check if this is the server end."""
        ...

    @property
    def is_shutdown(self) -> bool:
        """Check if the channel has been shutdown."""
        ...

    def wait_for_client(self) -> None:
        """Wait for a client to connect (server only).

        Raises:
            ConnectionError: If channel is already shutdown
        """
        ...

    def shutdown(self) -> None:
        """Signal the channel to shutdown.

        After calling this method:
        - New send/receive operations will raise ConnectionError
        - Pending operations may still complete
        - Use drain() to wait for pending operations
        """
        ...

    def drain(self) -> None:
        """Wait for all pending operations to complete."""
        ...

    def shutdown_timeout(self, timeout_ms: int) -> None:
        """Shutdown with a timeout.

        Combines shutdown() and drain() with a timeout.

        Args:
            timeout_ms: Timeout in milliseconds

        Raises:
            TimeoutError: If drain doesn't complete within timeout
        """
        ...

    def send(self, data: bytes) -> None:
        """Send bytes through the channel.

        Args:
            data: Data to send.

        Raises:
            ConnectionError: If channel is shutdown
        """
        ...

    def recv(self) -> bytes:
        """Receive bytes from the channel.

        Returns:
            Received data.

        Raises:
            ConnectionError: If channel is shutdown
        """
        ...

    def send_json(self, obj: Any) -> None:
        """Send a JSON-serializable object.

        Args:
            obj: Object to send (will be serialized to JSON).

        Raises:
            ConnectionError: If channel is shutdown
        """
        ...

    def recv_json(self) -> Any:
        """Receive a JSON object.

        Returns:
            Deserialized Python object.

        Raises:
            ConnectionError: If channel is shutdown
        """
        ...

# CLI Bridge classes

class CliBridgeConfig:
    """Configuration for CLI Bridge.

    Attributes:
        server_url: Socket path for the API server
        auto_register: Whether to auto-register as a task
        capture_stdout: Whether to capture stdout
        capture_stderr: Whether to capture stderr
    """

    def __init__(
        self,
        server_url: str | None = None,
        auto_register: bool = True,
        capture_stdout: bool = True,
        capture_stderr: bool = True,
    ) -> None:
        """Create a new configuration.

        Args:
            server_url: Socket path for the API server (default from env or system default)
            auto_register: Whether to auto-register as a task
            capture_stdout: Whether to capture stdout
            capture_stderr: Whether to capture stderr
        """
        ...

    @staticmethod
    def from_env() -> CliBridgeConfig:
        """Create configuration from environment variables.

        Environment variables:
        - IPCKIT_SERVER_URL: Socket path
        - IPCKIT_AUTO_REGISTER: "true" or "false"
        """
        ...

    @property
    def server_url(self) -> str:
        """Get the server URL."""
        ...

    @server_url.setter
    def server_url(self, url: str) -> None:
        """Set the server URL."""
        ...

    @property
    def auto_register(self) -> bool:
        """Get auto_register setting."""
        ...

    @auto_register.setter
    def auto_register(self, value: bool) -> None:
        """Set auto_register."""
        ...

class ProgressInfo:
    """Progress information parsed from output.

    Attributes:
        current: Current progress value
        total: Total value
        message: Optional progress message
        percentage: Calculated percentage (0-100)
    """

    def __init__(
        self,
        current: int,
        total: int,
        message: str | None = None,
    ) -> None:
        """Create a new progress info.

        Args:
            current: Current progress value
            total: Total value
            message: Optional message
        """
        ...

    @property
    def current(self) -> int:
        """Get current value."""
        ...

    @property
    def total(self) -> int:
        """Get total value."""
        ...

    @property
    def message(self) -> str | None:
        """Get message."""
        ...

    @property
    def percentage(self) -> int:
        """Get percentage (0-100)."""
        ...

class CliBridge:
    """CLI Bridge for integrating CLI tools with ipckit.

    This class allows CLI tools to communicate with frontends,
    report progress, and receive cancellation signals.

    Example:
        bridge = CliBridge.connect()
        bridge.register_task("My Task", "custom")

        for i in range(100):
            if bridge.is_cancelled:
                bridge.fail("Cancelled by user")
                return
            bridge.set_progress(i + 1, f"Step {i + 1}/100")

        bridge.complete({"success": True})
    """

    def __init__(self, config: CliBridgeConfig | None = None) -> None:
        """Create a new CLI bridge.

        Args:
            config: Configuration (default if not provided)
        """
        ...

    @staticmethod
    def connect() -> CliBridge:
        """Connect with default configuration (from environment)."""
        ...

    @staticmethod
    def connect_with_config(config: CliBridgeConfig) -> CliBridge:
        """Connect with the given configuration."""
        ...

    def register_task(self, name: str, task_type: str) -> str:
        """Register the current process as a task.

        Args:
            name: Task name
            task_type: Task type (e.g., "build", "upload", "custom")

        Returns:
            The task ID
        """
        ...

    @property
    def task_id(self) -> str | None:
        """Get the current task ID."""
        ...

    def set_progress(self, progress: int, message: str | None = None) -> None:
        """Set the progress (0-100).

        Args:
            progress: Progress value (0-100)
            message: Optional progress message
        """
        ...

    def log(self, level: str, message: str) -> None:
        """Log a message.

        Args:
            level: Log level ("info", "warn", "error", etc.)
            message: Log message
        """
        ...

    def stdout(self, line: str) -> None:
        """Send a stdout line."""
        ...

    def stderr(self, line: str) -> None:
        """Send a stderr line."""
        ...

    @property
    def is_cancelled(self) -> bool:
        """Check if cancellation has been requested."""
        ...

    def complete(self, result: Any) -> None:
        """Mark the task as complete.

        Args:
            result: Result data (will be serialized to JSON)
        """
        ...

    def fail(self, error: str) -> None:
        """Mark the task as failed.

        Args:
            error: Error message
        """
        ...

    def __enter__(self) -> CliBridge:
        """Enter context manager."""
        ...

    def __exit__(
        self,
        exc_type: type | None,
        exc_value: BaseException | None,
        traceback: Any | None,
    ) -> None:
        """Exit context manager (auto-fails on exception)."""
        ...

class CommandOutput:
    """Output from a wrapped command.

    Attributes:
        exit_code: Process exit code
        stdout: Captured stdout
        stderr: Captured stderr
        duration_ms: Duration in milliseconds
        success: True if exit_code is 0
    """

    @property
    def exit_code(self) -> int:
        """Get the exit code."""
        ...

    @property
    def stdout(self) -> str:
        """Get captured stdout."""
        ...

    @property
    def stderr(self) -> str:
        """Get captured stderr."""
        ...

    @property
    def duration_ms(self) -> int:
        """Get duration in milliseconds."""
        ...

    @property
    def success(self) -> bool:
        """Check if the command succeeded (exit_code == 0)."""
        ...

def wrap_command(
    args: list[str],
    task_name: str | None = None,
    task_type: str | None = None,
    cwd: str | None = None,
    env: dict[str, str] | None = None,
) -> CommandOutput:
    """Wrap a command for execution with CLI bridge integration.

    This function runs a subprocess and automatically:
    - Registers it as a task with the API server
    - Captures and forwards stdout/stderr
    - Parses progress from output
    - Reports completion/failure

    Args:
        args: Command and arguments as a list
        task_name: Name of the task (default: program name)
        task_type: Type of the task (default: "command")
        cwd: Working directory (optional)
        env: Environment variables (optional)

    Returns:
        CommandOutput with exit code, stdout, stderr, and duration

    Example:
        output = wrap_command(
            ["pip", "install", "-r", "requirements.txt"],
            task_name="Install Dependencies",
            task_type="install"
        )
        if output.success:
            print("Installation complete!")
    """
    ...

def parse_progress(line: str, parser_type: str = "all") -> ProgressInfo | None:
    """Parse progress from a line using built-in parsers.

    Args:
        line: The line to parse
        parser_type: Parser type:
            - "percentage": Matches "50%", "Progress: 75%", etc.
            - "fraction": Matches "5/10", "[3/4]", etc.
            - "progress_bar": Matches "[=====>    ] 50%"
            - "all": Try all parsers (default)

    Returns:
        ProgressInfo if progress was found, None otherwise

    Example:
        info = parse_progress("Downloading... 50%")
        if info:
            print(f"Progress: {info.percentage}%")
    """
    ...

# Metrics classes (Issue #10: Performance monitoring)

class ChannelMetrics:
    """Performance metrics for IPC channels.

    Tracks message counts, byte throughput, errors, latency, and queue depth.
    All operations are thread-safe using atomic counters.

    Example:
        metrics = ChannelMetrics()
        metrics.record_send(100)  # Record 100 bytes sent
        metrics.record_recv(50)   # Record 50 bytes received
        metrics.record_latency_us(150)  # Record 150µs latency

        print(f"Messages sent: {metrics.messages_sent}")
        print(f"Avg latency: {metrics.avg_latency_us}µs")
        print(metrics.to_prometheus('ipckit'))
    """

    def __init__(self) -> None:
        """Create a new metrics instance."""
        ...

    def record_send(self, bytes: int) -> None:
        """Record a message sent with the given byte count."""
        ...

    def record_recv(self, bytes: int) -> None:
        """Record a message received with the given byte count."""
        ...

    def record_send_error(self) -> None:
        """Record a send error."""
        ...

    def record_recv_error(self) -> None:
        """Record a receive error."""
        ...

    def record_latency_us(self, latency_us: int) -> None:
        """Record latency in microseconds."""
        ...

    def record_latency_ms(self, latency_ms: int) -> None:
        """Record latency in milliseconds."""
        ...

    def set_queue_depth(self, depth: int) -> None:
        """Update the current queue depth."""
        ...

    @property
    def messages_sent(self) -> int:
        """Get total messages sent."""
        ...

    @property
    def messages_received(self) -> int:
        """Get total messages received."""
        ...

    @property
    def bytes_sent(self) -> int:
        """Get total bytes sent."""
        ...

    @property
    def bytes_received(self) -> int:
        """Get total bytes received."""
        ...

    @property
    def send_errors(self) -> int:
        """Get send error count."""
        ...

    @property
    def receive_errors(self) -> int:
        """Get receive error count."""
        ...

    @property
    def queue_depth(self) -> int:
        """Get current queue depth."""
        ...

    @property
    def peak_queue_depth(self) -> int:
        """Get peak queue depth."""
        ...

    @property
    def avg_latency_us(self) -> int:
        """Get average latency in microseconds."""
        ...

    @property
    def min_latency_us(self) -> int | None:
        """Get minimum latency in microseconds."""
        ...

    @property
    def max_latency_us(self) -> int:
        """Get maximum latency in microseconds."""
        ...

    def latency_percentile(self, percentile: int) -> int:
        """Get latency percentile (e.g., 99 for p99)."""
        ...

    @property
    def elapsed_secs(self) -> float:
        """Get elapsed time since metrics started."""
        ...

    @property
    def send_throughput(self) -> float:
        """Get send throughput in messages per second."""
        ...

    @property
    def recv_throughput(self) -> float:
        """Get receive throughput in messages per second."""
        ...

    @property
    def send_bandwidth(self) -> float:
        """Get send bandwidth in bytes per second."""
        ...

    @property
    def recv_bandwidth(self) -> float:
        """Get receive bandwidth in bytes per second."""
        ...

    def reset(self) -> None:
        """Reset all metrics."""
        ...

    def snapshot(self) -> dict[str, Any]:
        """Get a snapshot of all metrics as a dict."""
        ...

    def to_json(self) -> str:
        """Export metrics as JSON string."""
        ...

    def to_json_pretty(self) -> str:
        """Export metrics as pretty JSON string."""
        ...

    def to_prometheus(self, prefix: str) -> str:
        """Export metrics in Prometheus format.

        Args:
            prefix: Metric name prefix (e.g., 'ipckit')

        Returns:
            Prometheus-formatted metrics string
        """
        ...

class MetricsSnapshot:
    """A point-in-time snapshot of channel metrics."""

    @property
    def messages_sent(self) -> int:
        """Total messages sent."""
        ...

    @property
    def messages_received(self) -> int:
        """Total messages received."""
        ...

    @property
    def bytes_sent(self) -> int:
        """Total bytes sent."""
        ...

    @property
    def bytes_received(self) -> int:
        """Total bytes received."""
        ...

    @property
    def send_errors(self) -> int:
        """Send error count."""
        ...

    @property
    def receive_errors(self) -> int:
        """Receive error count."""
        ...

    @property
    def queue_depth(self) -> int:
        """Current queue depth."""
        ...

    @property
    def peak_queue_depth(self) -> int:
        """Peak queue depth."""
        ...

    @property
    def avg_latency_us(self) -> int:
        """Average latency in microseconds."""
        ...

    @property
    def min_latency_us(self) -> int | None:
        """Minimum latency in microseconds."""
        ...

    @property
    def max_latency_us(self) -> int:
        """Maximum latency in microseconds."""
        ...

    @property
    def p50_latency_us(self) -> int:
        """50th percentile latency."""
        ...

    @property
    def p95_latency_us(self) -> int:
        """95th percentile latency."""
        ...

    @property
    def p99_latency_us(self) -> int:
        """99th percentile latency."""
        ...

    @property
    def elapsed_secs(self) -> float:
        """Elapsed time in seconds."""
        ...

    @property
    def send_throughput(self) -> float:
        """Send throughput (messages/second)."""
        ...

    @property
    def recv_throughput(self) -> float:
        """Receive throughput (messages/second)."""
        ...

    @property
    def send_bandwidth(self) -> float:
        """Send bandwidth (bytes/second)."""
        ...

    @property
    def recv_bandwidth(self) -> float:
        """Receive bandwidth (bytes/second)."""
        ...

    def to_dict(self) -> dict[str, Any]:
        """Convert to dict."""
        ...

# API Server classes (Issue #14: HTTP-over-Socket RESTful API)

class ApiServerConfig:
    """Configuration for API Server.

    Attributes:
        socket_path: Socket path for the server
        enable_cors: Whether to enable CORS
        cors_origins: List of allowed CORS origins
    """

    def __init__(
        self,
        socket_path: str | None = None,
        enable_cors: bool = True,
        cors_origins: list[str] | None = None,
    ) -> None:
        """Create a new configuration.

        Args:
            socket_path: Socket path for the server
            enable_cors: Whether to enable CORS (default: True)
            cors_origins: List of allowed origins (default: ["*"])
        """
        ...

    @property
    def socket_path(self) -> str:
        """Get the socket path."""
        ...

    @socket_path.setter
    def socket_path(self, path: str) -> None:
        """Set the socket path."""
        ...

    @property
    def enable_cors(self) -> bool:
        """Get CORS enabled setting."""
        ...

    @enable_cors.setter
    def enable_cors(self, value: bool) -> None:
        """Set CORS enabled."""
        ...

    @property
    def cors_origins(self) -> list[str]:
        """Get CORS allowed origins."""
        ...

    @cors_origins.setter
    def cors_origins(self, origins: list[str]) -> None:
        """Set CORS allowed origins."""
        ...

class Request:
    """HTTP Request object.

    Attributes:
        method: HTTP method (GET, POST, PUT, DELETE, etc.)
        path: Request path
        query: Query parameters as dict
        headers: Request headers as dict
        params: Path parameters as dict
        body: Request body (parsed JSON or None)
    """

    @property
    def method(self) -> str:
        """Get HTTP method."""
        ...

    @property
    def path(self) -> str:
        """Get request path."""
        ...

    @property
    def query(self) -> dict[str, str]:
        """Get query parameters."""
        ...

    @property
    def headers(self) -> dict[str, str]:
        """Get request headers."""
        ...

    @property
    def params(self) -> dict[str, str]:
        """Get path parameters."""
        ...

    @property
    def body(self) -> Any:
        """Get request body."""
        ...

    def query_param(self, name: str) -> str | None:
        """Get a query parameter by name."""
        ...

    def path_param(self, name: str) -> str | None:
        """Get a path parameter by name."""
        ...

    def header(self, name: str) -> str | None:
        """Get a header value by name (case-insensitive)."""
        ...

class Response:
    """HTTP Response object.

    Example:
        # Create responses
        resp = Response.ok({"data": [1, 2, 3]})
        resp = Response.created({"id": "new-item"})
        resp = Response.not_found()
        resp = Response.bad_request("Invalid input")

        # Custom response
        resp = Response(status=202)
        resp.set_header("X-Custom", "value")
        resp.set_json({"status": "accepted"})
    """

    def __init__(self, status: int = 200) -> None:
        """Create a new response with status code."""
        ...

    @staticmethod
    def ok(body: Any) -> Response:
        """Create a 200 OK response with JSON body."""
        ...

    @staticmethod
    def created(body: Any) -> Response:
        """Create a 201 Created response with JSON body."""
        ...

    @staticmethod
    def no_content() -> Response:
        """Create a 204 No Content response."""
        ...

    @staticmethod
    def bad_request(message: str) -> Response:
        """Create a 400 Bad Request response."""
        ...

    @staticmethod
    def not_found() -> Response:
        """Create a 404 Not Found response."""
        ...

    @staticmethod
    def internal_error(message: str) -> Response:
        """Create a 500 Internal Server Error response."""
        ...

    def set_header(self, key: str, value: str) -> None:
        """Set a response header."""
        ...

    def set_json(self, body: Any) -> None:
        """Set the response body as JSON."""
        ...

    @property
    def status(self) -> int:
        """Get the status code."""
        ...

class ApiClient:
    """Client for making HTTP requests to the API server.

    Example:
        # Connect without timeout (may block indefinitely)
        client = ApiClient.connect()

        # Connect with timeout (recommended for testing)
        client = ApiClient.connect_timeout(1000)  # 1 second timeout

        # Create with custom socket path and timeout
        client = ApiClient("/tmp/my.sock", timeout_ms=500)

        # GET request
        tasks = client.get('/v1/tasks')

        # POST request
        new_task = client.post('/v1/tasks', {'name': 'my-task'})

        # PUT request
        updated = client.put('/v1/tasks/123', {'name': 'updated'})

        # DELETE request
        client.delete('/v1/tasks/123')
    """

    def __init__(self, socket_path: str, timeout_ms: int | None = None) -> None:
        """Create a new API client.

        Args:
            socket_path: Path to the socket
            timeout_ms: Optional connection timeout in milliseconds.
                        If None, connection may block indefinitely.
        """
        ...

    @staticmethod
    def connect() -> ApiClient:
        """Connect to the default socket without timeout.

        Warning: This may block indefinitely if the server is not running.
        For testing, use connect_timeout() instead.
        """
        ...

    @staticmethod
    def connect_timeout(timeout_ms: int) -> ApiClient:
        """Connect to the default socket with a timeout.

        This is recommended for unit tests to avoid hanging.

        Args:
            timeout_ms: Connection timeout in milliseconds

        Returns:
            A new ApiClient instance

        Raises:
            RuntimeError: If connection times out or fails
        """
        ...

    def set_timeout(self, timeout_ms: int | None) -> None:
        """Set the connection timeout for future requests.

        Args:
            timeout_ms: Timeout in milliseconds, or None to disable timeout
        """
        ...

    def get_timeout(self) -> int | None:
        """Get the current connection timeout in milliseconds.

        Returns:
            Timeout in milliseconds, or None if no timeout is set
        """
        ...

    def get(self, path: str) -> Any:
        """Make a GET request.

        Args:
            path: Request path (e.g., '/v1/tasks')

        Returns:
            Response body as Python object

        Raises:
            RuntimeError: If connection fails or times out
        """
        ...

    def post(self, path: str, body: Any | None = None) -> Any:
        """Make a POST request.

        Args:
            path: Request path
            body: Request body (will be serialized to JSON)

        Returns:
            Response body as Python object

        Raises:
            RuntimeError: If connection fails or times out
        """
        ...

    def put(self, path: str, body: Any | None = None) -> Any:
        """Make a PUT request.

        Args:
            path: Request path
            body: Request body (will be serialized to JSON)

        Returns:
            Response body as Python object

        Raises:
            RuntimeError: If connection fails or times out
        """
        ...

    def delete(self, path: str) -> Any:
        """Make a DELETE request.

        Args:
            path: Request path

        Returns:
            Response body as Python object

        Raises:
            RuntimeError: If connection fails or times out
        """
        ...
