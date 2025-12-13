#!/usr/bin/env python3
"""
Example: Using IPC Channel for Message Passing

This example demonstrates the high-level IpcChannel API for
structured message passing between processes.
"""

import time
import threading
from ipckit import IpcChannel


def server(channel_name: str):
    """Server that handles JSON messages."""
    print(f"[Server] Creating channel: {channel_name}")
    channel = IpcChannel.create(channel_name)
    print("[Server] Waiting for client...")

    channel.wait_for_client()
    print("[Server] Client connected!")

    # Handle messages
    while True:
        data = channel.recv_json()
        print(f"[Server] Received: {data}")

        if data.get("type") == "exit":
            channel.send_json({"status": "goodbye"})
            break

        # Process and respond
        if data.get("type") == "ping":
            channel.send_json({"type": "pong", "id": data.get("id")})
        elif data.get("type") == "compute":
            result = sum(data.get("numbers", []))
            channel.send_json({"type": "result", "value": result})
        else:
            channel.send_json({"type": "error", "message": "Unknown type"})

    print("[Server] Done!")


def client(channel_name: str):
    """Client that sends JSON messages."""
    time.sleep(0.5)  # Wait for server

    print(f"[Client] Connecting to channel: {channel_name}")
    channel = IpcChannel.connect(channel_name)
    print("[Client] Connected!")

    # Send ping
    channel.send_json({"type": "ping", "id": 1})
    response = channel.recv_json()
    print(f"[Client] Ping response: {response}")

    # Send compute request
    channel.send_json({"type": "compute", "numbers": [1, 2, 3, 4, 5]})
    response = channel.recv_json()
    print(f"[Client] Compute response: {response}")

    # Send another ping
    channel.send_json({"type": "ping", "id": 2})
    response = channel.recv_json()
    print(f"[Client] Ping response: {response}")

    # Exit
    channel.send_json({"type": "exit"})
    response = channel.recv_json()
    print(f"[Client] Exit response: {response}")

    print("[Client] Done!")


def main():
    channel_name = f"example_channel_{time.time_ns()}"

    # Run server and client in separate threads
    server_thread = threading.Thread(target=server, args=(channel_name,))
    client_thread = threading.Thread(target=client, args=(channel_name,))

    server_thread.start()
    client_thread.start()

    server_thread.join()
    client_thread.join()

    print("\nDone!")


if __name__ == "__main__":
    main()
