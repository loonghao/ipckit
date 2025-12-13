#!/usr/bin/env python3
"""
Example: Using Named Pipes for IPC

This example demonstrates how to use named pipes for communication
between a server and client process.
"""

import sys
import time
import threading
from ipckit import NamedPipe


def server(pipe_name: str):
    """Server that receives messages from clients."""
    print(f"[Server] Creating pipe: {pipe_name}")
    pipe = NamedPipe.create(pipe_name)
    print(f"[Server] Waiting for client...")

    pipe.wait_for_client()
    print("[Server] Client connected!")

    # Receive message
    data = pipe.read(1024)
    print(f"[Server] Received: {data.decode()}")

    # Send response
    response = b"Hello from server!"
    pipe.write(response)
    print(f"[Server] Sent: {response.decode()}")


def client(pipe_name: str):
    """Client that sends messages to the server."""
    time.sleep(0.5)  # Wait for server to start

    print(f"[Client] Connecting to pipe: {pipe_name}")
    pipe = NamedPipe.connect(pipe_name)
    print("[Client] Connected!")

    # Send message
    message = b"Hello from client!"
    pipe.write(message)
    print(f"[Client] Sent: {message.decode()}")

    # Receive response
    data = pipe.read(1024)
    print(f"[Client] Received: {data.decode()}")


def main():
    pipe_name = "example_pipe"

    # Run server and client in separate threads
    server_thread = threading.Thread(target=server, args=(pipe_name,))
    client_thread = threading.Thread(target=client, args=(pipe_name,))

    server_thread.start()
    client_thread.start()

    server_thread.join()
    client_thread.join()

    print("\nDone!")


if __name__ == "__main__":
    main()
