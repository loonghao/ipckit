"""Tests for pipe functionality."""

import os
import sys
import threading
import time

import pytest


def test_anonymous_pipe():
    """Test anonymous pipe read/write."""
    from ipckit import AnonymousPipe

    pipe = AnonymousPipe()

    # Write in one thread, read in another
    def writer():
        time.sleep(0.1)
        pipe.write(b"Hello, Pipe!")

    thread = threading.Thread(target=writer)
    thread.start()

    data = pipe.read(1024)
    assert data == b"Hello, Pipe!"

    thread.join()


def test_anonymous_pipe_multiple_writes():
    """Test multiple writes to anonymous pipe."""
    from ipckit import AnonymousPipe

    pipe = AnonymousPipe()

    messages = [b"First", b"Second", b"Third"]

    def writer():
        time.sleep(0.05)
        for msg in messages:
            pipe.write(msg)
            time.sleep(0.01)

    thread = threading.Thread(target=writer)
    thread.start()

    received = b""
    for _ in range(3):
        received += pipe.read(1024)

    thread.join()

    for msg in messages:
        assert msg in received


def test_named_pipe_create_connect():
    """Test named pipe server/client."""
    from ipckit import NamedPipe

    pipe_name = f"test_pipe_{os.getpid()}"

    def server():
        server_pipe = NamedPipe.create(pipe_name)
        assert server_pipe.is_server
        server_pipe.wait_for_client()
        data = server_pipe.read(1024)
        assert data == b"Hello from client!"
        server_pipe.write(b"Hello from server!")

    def client():
        time.sleep(0.1)  # Wait for server to start
        client_pipe = NamedPipe.connect(pipe_name)
        assert not client_pipe.is_server
        client_pipe.write(b"Hello from client!")
        data = client_pipe.read(1024)
        assert data == b"Hello from server!"

    server_thread = threading.Thread(target=server)
    client_thread = threading.Thread(target=client)

    server_thread.start()
    client_thread.start()

    server_thread.join(timeout=5)
    client_thread.join(timeout=5)


def test_named_pipe_name():
    """Test named pipe name property."""
    from ipckit import NamedPipe

    pipe_name = f"test_name_pipe_{os.getpid()}"
    pipe = NamedPipe.create(pipe_name)

    # The name should contain the original name
    assert pipe_name in pipe.name or pipe.name.endswith(pipe_name)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
