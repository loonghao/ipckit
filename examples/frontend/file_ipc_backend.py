"""
File-based IPC Example - Python Backend

This demonstrates file-based IPC where:
- Python backend writes to: {channel_dir}/backend_to_frontend.json
- Python backend reads from: {channel_dir}/frontend_to_backend.json
- Frontend (any language) does the opposite

This is the simplest cross-platform IPC method - just read/write JSON files!

Usage:
    python file_ipc_backend.py

Then run the frontend (file_ipc_frontend.html or any app that can read/write files)
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


def handle_request(method: str, params: dict) -> dict:
    """Handle incoming requests from frontend"""
    
    if method == "ping":
        return {"pong": True, "timestamp": time.time()}
    
    elif method == "echo":
        return {"message": params.get("message", "")}
    
    elif method == "calculate":
        op = params.get("operation", "add")
        a = params.get("a", 0)
        b = params.get("b", 0)
        
        ops = {
            "add": a + b,
            "subtract": a - b,
            "multiply": a * b,
            "divide": a / b if b != 0 else float("inf"),
        }
        return {"result": ops.get(op, 0), "operation": op}
    
    elif method == "get_info":
        return {
            "version": ipckit.__version__,
            "python_version": sys.version,
            "platform": sys.platform,
        }
    
    elif method == "file_list":
        directory = params.get("path", ".")
        try:
            files = [f.name for f in Path(directory).iterdir()]
            return {"path": directory, "files": files}
        except Exception as e:
            raise ValueError(str(e))
    
    else:
        raise ValueError(f"Unknown method: {method}")


def main():
    # Create channel directory
    channel_dir = Path(__file__).parent / "ipc_channel"
    
    print("=" * 60)
    print("File-based IPC Backend")
    print("=" * 60)
    print(f"Channel directory: {channel_dir}")
    print()
    print("File structure:")
    print(f"  {channel_dir}/")
    print(f"  ├── backend_to_frontend.json  <- Backend writes here")
    print(f"  ├── frontend_to_backend.json  <- Frontend writes here")
    print(f"  └── .channel_info             <- Channel metadata")
    print()
    print("Frontend can be ANY language that reads/writes JSON files!")
    print("=" * 60)
    
    # Create backend channel
    channel = ipckit.FileChannel.backend(str(channel_dir))
    
    # Clear old messages
    channel.clear()
    
    # Send initial event to notify frontend that backend is ready
    channel.send_event("backend_ready", {
        "timestamp": time.time(),
        "available_methods": ["ping", "echo", "calculate", "get_info", "file_list"]
    })
    
    print("\n[Backend] Ready and waiting for frontend messages...")
    print("[Backend] Press Ctrl+C to stop\n")
    
    try:
        while True:
            # Check for new messages
            messages = channel.recv()
            
            for msg in messages:
                msg_type = msg.get("type")
                msg_id = msg.get("id")
                method = msg.get("method")
                payload = msg.get("payload", {})
                
                if msg_type == "request":
                    print(f"[Backend] Received request: {method}({json.dumps(payload)})")
                    
                    try:
                        result = handle_request(method, payload)
                        channel.send_response(msg_id, result)
                        print(f"[Backend] Sent response: {json.dumps(result)}")
                    except Exception as e:
                        channel.send_error(msg_id, str(e))
                        print(f"[Backend] Sent error: {e}")
                
                elif msg_type == "event":
                    print(f"[Backend] Received event: {method} - {json.dumps(payload)}")
            
            # Poll interval
            time.sleep(0.1)
    
    except KeyboardInterrupt:
        print("\n[Backend] Shutting down...")
        channel.send_event("backend_shutdown", {"timestamp": time.time()})


if __name__ == "__main__":
    main()
