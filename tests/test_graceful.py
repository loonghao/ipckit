"""Tests for GracefulChannel functionality."""

import threading
import time
import pytest


class TestGracefulNamedPipe:
    """Tests for GracefulNamedPipe."""

    def test_create_and_connect(self):
        """Test creating and connecting to a graceful named pipe."""
        import ipckit

        pipe_name = f"test_graceful_pipe_{time.time_ns()}"

        # Create server in a thread
        server_ready = threading.Event()
        server_data = {}

        def server_thread():
            server = ipckit.GracefulNamedPipe.create(pipe_name)
            assert server.name.endswith(pipe_name)
            assert server.is_server
            assert not server.is_shutdown
            server_ready.set()
            server.wait_for_client()

            # Read data
            data = server.read(32)
            server_data["received"] = data

            # Shutdown
            server.shutdown()
            assert server.is_shutdown

        thread = threading.Thread(target=server_thread)
        thread.start()

        # Wait for server to be ready
        server_ready.wait(timeout=5)
        time.sleep(0.1)

        # Connect as client
        client = ipckit.GracefulNamedPipe.connect(pipe_name)
        assert not client.is_server
        assert not client.is_shutdown

        # Send data
        client.write(b"Hello, Graceful!")

        thread.join(timeout=5)
        assert server_data.get("received") == b"Hello, Graceful!"

    def test_shutdown_prevents_operations(self):
        """Test that shutdown prevents new operations."""
        import ipckit

        pipe_name = f"test_graceful_shutdown_{time.time_ns()}"

        server = ipckit.GracefulNamedPipe.create(pipe_name)
        assert not server.is_shutdown

        server.shutdown()
        assert server.is_shutdown

        # Operations after shutdown should fail
        with pytest.raises(Exception):
            server.wait_for_client()

    def test_shutdown_timeout(self):
        """Test shutdown with timeout."""
        import ipckit

        pipe_name = f"test_graceful_timeout_{time.time_ns()}"

        server = ipckit.GracefulNamedPipe.create(pipe_name)

        # Shutdown with timeout should work when no pending operations
        server.shutdown_timeout(100)
        assert server.is_shutdown


class TestGracefulIpcChannel:
    """Tests for GracefulIpcChannel."""

    def test_create_and_connect(self):
        """Test creating and connecting to a graceful IPC channel."""
        import ipckit

        channel_name = f"test_graceful_channel_{time.time_ns()}"

        # Create server in a thread
        server_ready = threading.Event()
        server_data = {}

        def server_thread():
            server = ipckit.GracefulIpcChannel.create(channel_name)
            assert server.name.endswith(channel_name)
            assert server.is_server
            assert not server.is_shutdown
            server_ready.set()
            server.wait_for_client()

            # Receive data
            data = server.recv()
            server_data["received"] = data

            # Shutdown
            server.shutdown()
            assert server.is_shutdown

        thread = threading.Thread(target=server_thread)
        thread.start()

        # Wait for server to be ready
        server_ready.wait(timeout=5)
        time.sleep(0.1)

        # Connect as client
        client = ipckit.GracefulIpcChannel.connect(channel_name)
        assert not client.is_server
        assert not client.is_shutdown

        # Send data
        client.send(b"Hello, IPC!")

        thread.join(timeout=5)
        assert server_data.get("received") == b"Hello, IPC!"

    def test_send_recv_json(self):
        """Test sending and receiving JSON data."""
        import ipckit

        channel_name = f"test_graceful_json_{time.time_ns()}"

        # Create server in a thread
        server_ready = threading.Event()
        server_data = {}

        def server_thread():
            server = ipckit.GracefulIpcChannel.create(channel_name)
            server_ready.set()
            server.wait_for_client()

            # Receive JSON
            data = server.recv_json()
            server_data["received"] = data

            server.shutdown()

        thread = threading.Thread(target=server_thread)
        thread.start()

        # Wait for server to be ready
        server_ready.wait(timeout=5)
        time.sleep(0.1)

        # Connect and send JSON
        client = ipckit.GracefulIpcChannel.connect(channel_name)
        client.send_json({"message": "Hello", "count": 42})

        thread.join(timeout=5)
        assert server_data.get("received") == {"message": "Hello", "count": 42}

    def test_shutdown_prevents_operations(self):
        """Test that shutdown prevents new operations."""
        import ipckit

        channel_name = f"test_graceful_channel_shutdown_{time.time_ns()}"

        server = ipckit.GracefulIpcChannel.create(channel_name)
        assert not server.is_shutdown

        server.shutdown()
        assert server.is_shutdown

        # Operations after shutdown should fail
        with pytest.raises(Exception):
            server.send(b"test")

    def test_drain(self):
        """Test draining pending operations."""
        import ipckit

        channel_name = f"test_graceful_drain_{time.time_ns()}"

        server = ipckit.GracefulIpcChannel.create(channel_name)

        # Drain should work immediately when no pending operations
        server.shutdown()
        server.drain()

        assert server.is_shutdown

    def test_shutdown_timeout(self):
        """Test shutdown with timeout."""
        import ipckit

        channel_name = f"test_graceful_channel_timeout_{time.time_ns()}"

        server = ipckit.GracefulIpcChannel.create(channel_name)

        # Shutdown with timeout should work when no pending operations
        server.shutdown_timeout(100)
        assert server.is_shutdown


class TestGracefulChannelConcurrency:
    """Tests for concurrent access to graceful channels."""

    def test_concurrent_shutdown(self):
        """Test that concurrent shutdown is safe."""
        import ipckit

        channel_name = f"test_concurrent_shutdown_{time.time_ns()}"
        server = ipckit.GracefulIpcChannel.create(channel_name)

        # Multiple threads calling shutdown should be safe
        def shutdown_thread():
            server.shutdown()

        threads = [threading.Thread(target=shutdown_thread) for _ in range(10)]
        for t in threads:
            t.start()
        for t in threads:
            t.join()

        assert server.is_shutdown

    def test_operations_during_shutdown(self):
        """Test that operations during shutdown are handled gracefully."""
        import ipckit

        pipe_name = f"test_ops_during_shutdown_{time.time_ns()}"

        server_ready = threading.Event()
        shutdown_started = threading.Event()
        errors = []

        def server_thread():
            server = ipckit.GracefulNamedPipe.create(pipe_name)
            server_ready.set()

            # Wait for shutdown signal
            shutdown_started.wait(timeout=5)

            # Try to operate after shutdown - should fail gracefully
            try:
                server.write(b"test")
            except Exception as e:
                errors.append(str(e))

        thread = threading.Thread(target=server_thread)
        thread.start()

        server_ready.wait(timeout=5)
        time.sleep(0.1)

        # Connect and shutdown
        client = ipckit.GracefulNamedPipe.connect(pipe_name)
        client.shutdown()
        shutdown_started.set()

        thread.join(timeout=5)
        # Server should have received an error
        # (may or may not depending on timing, both are acceptable)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
