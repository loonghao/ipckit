"""
File-based IPC Example - Python Frontend (Simulating a desktop app)

This demonstrates the frontend side of file-based IPC.
In a real application, this would be:
- JavaScript in Electron (using fs module)
- Rust in Tauri (using std::fs)
- Any language with file I/O capability

Usage:
    # Terminal 1: Start backend
    python file_ipc_backend.py
    
    # Terminal 2: Run this frontend
    python file_ipc_frontend.py
"""

import json
import sys
import time
from pathlib import Path

try:
    import ipckit
except ImportError:
    print("Error: ipckit not installed. Run: maturin develop --features python-bindings,ext-module")
    sys.exit(1)


def main():
    # Channel directory (same as backend)
    channel_dir = Path(__file__).parent / "ipc_channel"
    
    print("=" * 60)
    print("File-based IPC Frontend")
    print("=" * 60)
    print(f"Channel directory: {channel_dir}")
    print()
    
    # Create frontend channel
    channel = ipckit.FileChannel.frontend(str(channel_dir))
    
    print("[Frontend] Connected to channel")
    print("[Frontend] Waiting for backend_ready event...")
    
    # Wait for backend to be ready
    timeout = 10  # seconds
    start = time.time()
    backend_ready = False
    
    while time.time() - start < timeout:
        messages = channel.recv()
        for msg in messages:
            if msg.get("type") == "event" and msg.get("method") == "backend_ready":
                print(f"[Frontend] Backend is ready!")
                print(f"[Frontend] Available methods: {msg['payload'].get('available_methods', [])}")
                backend_ready = True
                break
        if backend_ready:
            break
        time.sleep(0.1)
    
    if not backend_ready:
        print("[Frontend] Warning: Backend ready event not received, continuing anyway...")
    
    print("\n" + "=" * 60)
    print("Running IPC Tests")
    print("=" * 60 + "\n")
    
    # Test 1: Ping
    print("[Test 1] Ping...")
    request_id = channel.send_request("ping", {})
    response = channel.wait_response(request_id, timeout_ms=5000)
    print(f"  Response: {json.dumps(response['payload'])}")
    
    # Test 2: Echo
    print("\n[Test 2] Echo...")
    request_id = channel.send_request("echo", {"message": "Hello from frontend!"})
    response = channel.wait_response(request_id, timeout_ms=5000)
    print(f"  Response: {json.dumps(response['payload'])}")
    
    # Test 3: Calculate
    print("\n[Test 3] Calculate...")
    for op, a, b in [("add", 10, 5), ("multiply", 7, 6), ("divide", 100, 4)]:
        request_id = channel.send_request("calculate", {"operation": op, "a": a, "b": b})
        response = channel.wait_response(request_id, timeout_ms=5000)
        result = response['payload']['result']
        print(f"  {a} {op} {b} = {result}")
    
    # Test 4: Get Info
    print("\n[Test 4] Get backend info...")
    request_id = channel.send_request("get_info", {})
    response = channel.wait_response(request_id, timeout_ms=5000)
    print(f"  Backend info: {json.dumps(response['payload'], indent=2)}")
    
    # Test 5: File list
    print("\n[Test 5] List files in current directory...")
    request_id = channel.send_request("file_list", {"path": "."})
    response = channel.wait_response(request_id, timeout_ms=5000)
    files = response['payload'].get('files', [])[:5]  # Show first 5
    print(f"  Files: {files}...")
    
    # Test 6: Error handling
    print("\n[Test 6] Error handling (unknown method)...")
    request_id = channel.send_request("unknown_method", {})
    response = channel.wait_response(request_id, timeout_ms=5000)
    if response.get("error"):
        print(f"  Expected error: {response['error']}")
    
    # Send goodbye event
    print("\n[Frontend] Sending goodbye event...")
    channel.send_event("frontend_goodbye", {"message": "Tests completed!"})
    
    print("\n" + "=" * 60)
    print("All tests completed!")
    print("=" * 60)


if __name__ == "__main__":
    main()
