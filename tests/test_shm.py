"""Tests for shared memory functionality."""

import os
import threading
import time

import pytest


def test_shared_memory_create():
    """Test shared memory creation."""
    from ipckit import SharedMemory

    name = f"test_shm_{os.getpid()}"
    size = 1024

    shm = SharedMemory.create(name, size)
    assert shm.name == name or name in shm.name
    assert shm.size == size
    assert shm.is_owner


def test_shared_memory_write_read():
    """Test shared memory write and read."""
    from ipckit import SharedMemory

    name = f"test_shm_rw_{os.getpid()}"
    shm = SharedMemory.create(name, 1024)

    data = b"Hello, Shared Memory!"
    shm.write(0, data)

    read_data = shm.read(0, len(data))
    assert read_data == data


def test_shared_memory_offset():
    """Test shared memory read/write with offset."""
    from ipckit import SharedMemory

    name = f"test_shm_offset_{os.getpid()}"
    shm = SharedMemory.create(name, 1024)

    # Write at different offsets
    shm.write(0, b"AAAA")
    shm.write(100, b"BBBB")
    shm.write(200, b"CCCC")

    assert shm.read(0, 4) == b"AAAA"
    assert shm.read(100, 4) == b"BBBB"
    assert shm.read(200, 4) == b"CCCC"


def test_shared_memory_read_all():
    """Test reading all shared memory."""
    from ipckit import SharedMemory

    name = f"test_shm_all_{os.getpid()}"
    size = 100
    shm = SharedMemory.create(name, size)

    data = b"X" * size
    shm.write(0, data)

    read_data = shm.read_all()
    assert read_data == data


def test_shared_memory_boundary_error():
    """Test shared memory boundary checking."""
    from ipckit import SharedMemory

    name = f"test_shm_boundary_{os.getpid()}"
    shm = SharedMemory.create(name, 100)

    # Writing beyond boundary should fail
    with pytest.raises(Exception):
        shm.write(90, b"X" * 20)

    # Reading beyond boundary should fail
    with pytest.raises(Exception):
        shm.read(90, 20)


def test_shared_memory_open():
    """Test opening existing shared memory."""
    from ipckit import SharedMemory

    name = f"test_shm_open_{os.getpid()}"
    size = 1024

    # Create shared memory
    shm1 = SharedMemory.create(name, size)
    shm1.write(0, b"Shared data!")

    # Open in another "process" (simulated)
    shm2 = SharedMemory.open(name)
    assert not shm2.is_owner

    data = shm2.read(0, 12)
    assert data == b"Shared data!"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
