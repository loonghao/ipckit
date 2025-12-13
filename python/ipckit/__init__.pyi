"""Type stubs for ipckit"""

from typing import Any, Dict, List, Optional, Union

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
    def create(name: str) -> "NamedPipe":
        """Create a new named pipe server.

        Args:
            name: Pipe name.

        Returns:
            A new NamedPipe instance.
        """
        ...

    @staticmethod
    def connect(name: str) -> "NamedPipe":
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
    def create(name: str, size: int) -> "SharedMemory":
        """Create a new shared memory region.

        Args:
            name: Unique name for the shared memory.
            size: Size in bytes.

        Returns:
            A new SharedMemory instance.
        """
        ...

    @staticmethod
    def open(name: str) -> "SharedMemory":
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
    def create(name: str) -> "IpcChannel":
        """Create a new IPC channel server.

        Args:
            name: Channel name.

        Returns:
            A new IpcChannel instance.
        """
        ...

    @staticmethod
    def connect(name: str) -> "IpcChannel":
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
    def backend(dir: str) -> "FileChannel":
        """Create a backend-side file channel.
        
        Args:
            dir: Directory for channel files (will be created if not exists)
        
        Returns:
            A new FileChannel instance for backend use.
        """
        ...

    @staticmethod
    def frontend(dir: str) -> "FileChannel":
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

    def recv(self) -> List[Dict[str, Any]]:
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

    def recv_one(self) -> Optional[Dict[str, Any]]:
        """Receive a single new message (non-blocking).
        
        Returns:
            Message dict if available, None otherwise
        """
        ...

    def wait_response(self, request_id: str, timeout_ms: int) -> Dict[str, Any]:
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
    def create(name: str) -> "GracefulNamedPipe":
        """Create a new named pipe server with graceful shutdown.

        Args:
            name: Pipe name.

        Returns:
            A new GracefulNamedPipe instance.
        """
        ...

    @staticmethod
    def connect(name: str) -> "GracefulNamedPipe":
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
    def create(name: str) -> "GracefulIpcChannel":
        """Create a new IPC channel server with graceful shutdown.

        Args:
            name: Channel name.

        Returns:
            A new GracefulIpcChannel instance.
        """
        ...

    @staticmethod
    def connect(name: str) -> "GracefulIpcChannel":
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
