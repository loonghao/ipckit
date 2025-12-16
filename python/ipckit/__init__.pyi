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
