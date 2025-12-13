"""Tests for IPC channel functionality."""

import os
import threading
import time

import pytest


def test_channel_bytes():
    """Test channel send/recv bytes."""
    from ipckit import IpcChannel

    name = f"test_channel_{os.getpid()}"

    def server():
        channel = IpcChannel.create(name)
        channel.wait_for_client()
        data = channel.recv()
        assert data == b"Hello, Channel!"
        channel.send(b"Response!")

    def client():
        time.sleep(0.1)
        channel = IpcChannel.connect(name)
        channel.send(b"Hello, Channel!")
        data = channel.recv()
        assert data == b"Response!"

    server_thread = threading.Thread(target=server)
    client_thread = threading.Thread(target=client)

    server_thread.start()
    client_thread.start()

    server_thread.join(timeout=5)
    client_thread.join(timeout=5)

    assert not server_thread.is_alive(), "Server thread timed out"
    assert not client_thread.is_alive(), "Client thread timed out"


def test_channel_json():
    """Test channel send/recv JSON."""
    from ipckit import IpcChannel

    name = f"test_channel_json_{os.getpid()}"

    test_data = {
        "message": "Hello, JSON!",
        "number": 42,
        "list": [1, 2, 3],
        "nested": {"key": "value"},
    }

    def server():
        channel = IpcChannel.create(name)
        channel.wait_for_client()
        data = channel.recv_json()
        assert data == test_data
        channel.send_json({"status": "ok"})

    def client():
        time.sleep(0.1)
        channel = IpcChannel.connect(name)
        channel.send_json(test_data)
        response = channel.recv_json()
        assert response == {"status": "ok"}

    server_thread = threading.Thread(target=server)
    client_thread = threading.Thread(target=client)

    server_thread.start()
    client_thread.start()

    server_thread.join(timeout=5)
    client_thread.join(timeout=5)

    assert not server_thread.is_alive(), "Server thread timed out"
    assert not client_thread.is_alive(), "Client thread timed out"


def test_channel_large_message():
    """Test channel with large messages."""
    from ipckit import IpcChannel

    name = f"test_channel_large_{os.getpid()}"

    # 1 MB message
    large_data = b"X" * (1024 * 1024)

    def server():
        channel = IpcChannel.create(name)
        channel.wait_for_client()
        data = channel.recv()
        assert data == large_data
        channel.send(b"OK")

    def client():
        time.sleep(0.1)
        channel = IpcChannel.connect(name)
        channel.send(large_data)
        response = channel.recv()
        assert response == b"OK"

    server_thread = threading.Thread(target=server)
    client_thread = threading.Thread(target=client)

    server_thread.start()
    client_thread.start()

    server_thread.join(timeout=10)
    client_thread.join(timeout=10)

    assert not server_thread.is_alive(), "Server thread timed out"
    assert not client_thread.is_alive(), "Client thread timed out"


def test_channel_multiple_messages():
    """Test channel with multiple messages."""
    from ipckit import IpcChannel

    name = f"test_channel_multi_{os.getpid()}"
    messages = [b"First", b"Second", b"Third", b"Fourth", b"Fifth"]

    def server():
        channel = IpcChannel.create(name)
        channel.wait_for_client()
        for expected in messages:
            data = channel.recv()
            assert data == expected
        channel.send(b"Done")

    def client():
        time.sleep(0.1)
        channel = IpcChannel.connect(name)
        for msg in messages:
            channel.send(msg)
        response = channel.recv()
        assert response == b"Done"

    server_thread = threading.Thread(target=server)
    client_thread = threading.Thread(target=client)

    server_thread.start()
    client_thread.start()

    server_thread.join(timeout=5)
    client_thread.join(timeout=5)

    assert not server_thread.is_alive(), "Server thread timed out"
    assert not client_thread.is_alive(), "Client thread timed out"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
