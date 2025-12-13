#!/usr/bin/env python3
"""
Example: Using Shared Memory for IPC

This example demonstrates how to use shared memory for fast data
exchange between processes.
"""

import time
import threading
from ipckit import SharedMemory


def writer(shm_name: str):
    """Writer that creates and writes to shared memory."""
    print(f"[Writer] Creating shared memory: {shm_name}")
    shm = SharedMemory.create(shm_name, 1024)
    print(f"[Writer] Size: {shm.size} bytes")

    messages = [
        b"Message 1: Hello!",
        b"Message 2: World!",
        b"Message 3: Done!",
    ]

    for i, msg in enumerate(messages):
        # Write message length at offset 0
        length = len(msg).to_bytes(4, 'little')
        shm.write(0, length)

        # Write message content at offset 4
        shm.write(4, msg)
        print(f"[Writer] Wrote: {msg.decode()}")

        time.sleep(0.5)

    # Signal end with zero length
    shm.write(0, (0).to_bytes(4, 'little'))
    print("[Writer] Done!")


def reader(shm_name: str):
    """Reader that opens and reads from shared memory."""
    time.sleep(0.2)  # Wait for writer to create

    print(f"[Reader] Opening shared memory: {shm_name}")
    shm = SharedMemory.open(shm_name)
    print(f"[Reader] Size: {shm.size} bytes")

    last_msg = b""
    while True:
        # Read message length
        length_bytes = shm.read(0, 4)
        length = int.from_bytes(length_bytes, 'little')

        if length == 0:
            print("[Reader] End signal received!")
            break

        # Read message content
        msg = shm.read(4, length)
        if msg != last_msg:
            print(f"[Reader] Read: {msg.decode()}")
            last_msg = msg

        time.sleep(0.1)

    print("[Reader] Done!")


def main():
    shm_name = f"example_shm_{time.time_ns()}"

    # Run writer and reader in separate threads
    writer_thread = threading.Thread(target=writer, args=(shm_name,))
    reader_thread = threading.Thread(target=reader, args=(shm_name,))

    writer_thread.start()
    reader_thread.start()

    writer_thread.join()
    reader_thread.join()

    print("\nDone!")


if __name__ == "__main__":
    main()
