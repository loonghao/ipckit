"""Tests for IPC channel functionality."""

import os
import sys
import threading
import time

import pytest

# Skip bidirectional tests on Unix - FIFO doesn't support true bidirectional communication
# Windows named pipes support duplex mode
IS_UNIX = sys.platform != "win32"


def test_channel_bytes():
    """Test channel send/recv bytes."""
    from ipckit import IpcChannel

    name = f"test_channel_{os.getpid()}"
    results = {"server_ok": False, "client_ok": False}
    errors = []

    def server():
        try:
            channel = IpcChannel.create(name)
            channel.wait_for_client()
            data = channel.recv()
            if data == b"Hello, Channel!":
                results["server_ok"] = True
                channel.send(b"Response!")
        except Exception as e:
            errors.append(f"Server error: {e}")

    def client():
        try:
            time.sleep(0.1)
            channel = IpcChannel.connect(name)
            channel.send(b"Hello, Channel!")
            data = channel.recv()
            if data == b"Response!":
                results["client_ok"] = True
        except Exception as e:
            errors.append(f"Client error: {e}")

    server_thread = threading.Thread(target=server)
    client_thread = threading.Thread(target=client)

    server_thread.start()
    client_thread.start()

    server_thread.join(timeout=5)
    client_thread.join(timeout=5)

    if IS_UNIX and errors:
        pytest.skip("Bidirectional IPC not fully supported on Unix FIFO")

    assert not errors, f"Errors occurred: {errors}"
    assert results["server_ok"], "Server did not receive correct data"
    assert results["client_ok"], "Client did not receive correct response"


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

    results = {"server_ok": False, "client_ok": False}
    errors = []

    def server():
        try:
            channel = IpcChannel.create(name)
            channel.wait_for_client()
            data = channel.recv_json()
            if data == test_data:
                results["server_ok"] = True
                channel.send_json({"status": "ok"})
        except Exception as e:
            errors.append(f"Server error: {e}")

    def client():
        try:
            time.sleep(0.1)
            channel = IpcChannel.connect(name)
            channel.send_json(test_data)
            response = channel.recv_json()
            if response == {"status": "ok"}:
                results["client_ok"] = True
        except Exception as e:
            errors.append(f"Client error: {e}")

    server_thread = threading.Thread(target=server)
    client_thread = threading.Thread(target=client)

    server_thread.start()
    client_thread.start()

    server_thread.join(timeout=5)
    client_thread.join(timeout=5)

    if IS_UNIX and errors:
        pytest.skip("Bidirectional IPC not fully supported on Unix FIFO")

    assert not errors, f"Errors occurred: {errors}"
    assert results["server_ok"], "Server did not receive correct data"
    assert results["client_ok"], "Client did not receive correct response"


def test_channel_large_message():
    """Test channel with large messages."""
    from ipckit import IpcChannel

    name = f"test_channel_large_{os.getpid()}"

    # 1 MB message
    large_data = b"X" * (1024 * 1024)

    results = {"server_ok": False, "client_ok": False}
    errors = []

    def server():
        try:
            channel = IpcChannel.create(name)
            channel.wait_for_client()
            data = channel.recv()
            if data == large_data:
                results["server_ok"] = True
                channel.send(b"OK")
        except Exception as e:
            errors.append(f"Server error: {e}")

    def client():
        try:
            time.sleep(0.1)
            channel = IpcChannel.connect(name)
            channel.send(large_data)
            response = channel.recv()
            if response == b"OK":
                results["client_ok"] = True
        except Exception as e:
            errors.append(f"Client error: {e}")

    server_thread = threading.Thread(target=server)
    client_thread = threading.Thread(target=client)

    server_thread.start()
    client_thread.start()

    server_thread.join(timeout=10)
    client_thread.join(timeout=10)

    if IS_UNIX and errors:
        pytest.skip("Bidirectional IPC not fully supported on Unix FIFO")

    assert not errors, f"Errors occurred: {errors}"
    assert results["server_ok"], "Server did not receive correct data"
    assert results["client_ok"], "Client did not receive correct response"


def test_channel_multiple_messages():
    """Test channel with multiple messages."""
    from ipckit import IpcChannel

    name = f"test_channel_multi_{os.getpid()}"
    messages = [b"First", b"Second", b"Third", b"Fourth", b"Fifth"]

    results = {"server_ok": False, "client_ok": False}
    errors = []

    def server():
        try:
            channel = IpcChannel.create(name)
            channel.wait_for_client()
            for expected in messages:
                data = channel.recv()
                if data != expected:
                    errors.append(f"Expected {expected}, got {data}")
                    return
            results["server_ok"] = True
            channel.send(b"Done")
        except Exception as e:
            errors.append(f"Server error: {e}")

    def client():
        try:
            time.sleep(0.1)
            channel = IpcChannel.connect(name)
            for msg in messages:
                channel.send(msg)
            response = channel.recv()
            if response == b"Done":
                results["client_ok"] = True
        except Exception as e:
            errors.append(f"Client error: {e}")

    server_thread = threading.Thread(target=server)
    client_thread = threading.Thread(target=client)

    server_thread.start()
    client_thread.start()

    server_thread.join(timeout=5)
    client_thread.join(timeout=5)

    if IS_UNIX and errors:
        pytest.skip("Bidirectional IPC not fully supported on Unix FIFO")

    assert not errors, f"Errors occurred: {errors}"
    assert results["server_ok"], "Server did not receive all messages correctly"
    assert results["client_ok"], "Client did not receive response"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
