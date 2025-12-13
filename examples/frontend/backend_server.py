"""
ipckit Frontend-Backend IPC Example - Python Backend Server

This example demonstrates how to use ipckit for frontend-backend communication.
The backend runs as a Python process and communicates with the frontend via:
1. Named Pipe (for desktop WebView apps like Electron/Tauri)
2. WebSocket bridge (for web browsers)

Usage:
    python backend_server.py --mode pipe     # For desktop apps (Named Pipe)
    python backend_server.py --mode websocket # For web browsers (WebSocket)
"""

import argparse
import json
import sys
import threading
import time
from dataclasses import dataclass
from typing import Any, Callable, Dict, Optional

# Try to import ipckit
# For development, run: maturin develop --features python-bindings,ext-module
try:
    import ipckit
except ImportError:
    print("Error: ipckit not installed. Run: maturin develop --features python-bindings,ext-module")
    print("Or install from PyPI: pip install ipckit")
    sys.exit(1)


@dataclass
class IpcMessage:
    """Standard IPC message format for frontend-backend communication"""
    id: str           # Unique message ID for request-response matching
    method: str       # RPC method name
    params: Dict[str, Any]  # Method parameters
    
    def to_dict(self) -> dict:
        return {"id": self.id, "method": self.method, "params": self.params}
    
    @classmethod
    def from_dict(cls, data: dict) -> "IpcMessage":
        return cls(
            id=data.get("id", ""),
            method=data.get("method", ""),
            params=data.get("params", {})
        )


@dataclass
class IpcResponse:
    """Standard IPC response format"""
    id: str           # Matching request ID
    result: Any       # Success result
    error: Optional[str] = None  # Error message if failed
    
    def to_dict(self) -> dict:
        return {"id": self.id, "result": self.result, "error": self.error}


class IpcBackend:
    """
    IPC Backend Server for frontend communication.
    
    Supports multiple transport modes:
    - Named Pipe: For desktop WebView apps (Electron, Tauri, CEF)
    - WebSocket: For web browsers
    """
    
    def __init__(self, channel_name: str = "ipckit_frontend"):
        self.channel_name = channel_name
        self.handlers: Dict[str, Callable] = {}
        self.running = False
        self._channel = None
        
        # Register built-in handlers
        self.register("ping", self._handle_ping)
        self.register("echo", self._handle_echo)
        self.register("get_info", self._handle_get_info)
    
    def register(self, method: str, handler: Callable):
        """Register an RPC handler for a method"""
        self.handlers[method] = handler
    
    def _handle_ping(self, params: dict) -> dict:
        """Built-in ping handler"""
        return {"pong": True, "timestamp": time.time()}
    
    def _handle_echo(self, params: dict) -> Any:
        """Built-in echo handler"""
        return params.get("message", "")
    
    def _handle_get_info(self, params: dict) -> dict:
        """Get backend information"""
        return {
            "version": getattr(ipckit, "__version__", "0.1.0"),
            "python_version": sys.version,
            "platform": sys.platform,
            "channel_name": self.channel_name,
        }
    
    def _process_message(self, msg: IpcMessage) -> IpcResponse:
        """Process an incoming message and return response"""
        handler = self.handlers.get(msg.method)
        
        if handler is None:
            return IpcResponse(
                id=msg.id,
                result=None,
                error=f"Unknown method: {msg.method}"
            )
        
        try:
            result = handler(msg.params)
            return IpcResponse(id=msg.id, result=result)
        except Exception as e:
            return IpcResponse(id=msg.id, result=None, error=str(e))
    
    def run_pipe_server(self):
        """Run the IPC server using Named Pipe"""
        print(f"[Backend] Starting Named Pipe server: {self.channel_name}")
        
        self._channel = ipckit.IpcChannel.create(self.channel_name)
        self.running = True
        
        print(f"[Backend] Waiting for frontend to connect...")
        self._channel.wait_for_client()
        print(f"[Backend] Frontend connected!")
        
        while self.running:
            try:
                # Receive JSON message from frontend
                data = self._channel.recv_json()
                msg = IpcMessage.from_dict(data)
                
                print(f"[Backend] Received: {msg.method}({msg.params})")
                
                # Process and send response
                response = self._process_message(msg)
                self._channel.send_json(response.to_dict())
                
                print(f"[Backend] Sent response: {response.result}")
                
            except Exception as e:
                if self.running:
                    print(f"[Backend] Error: {e}")
                break
        
        print("[Backend] Server stopped")
    
    def run_websocket_server(self, host: str = "localhost", port: int = 8765):
        """Run the IPC server using WebSocket (for browser frontend)"""
        try:
            import asyncio
            import websockets
        except ImportError:
            print("Error: websockets package required. Install with: pip install websockets")
            sys.exit(1)
        
        async def handle_client(websocket):
            print(f"[Backend] WebSocket client connected")
            
            try:
                async for message in websocket:
                    data = json.loads(message)
                    msg = IpcMessage.from_dict(data)
                    
                    print(f"[Backend] Received: {msg.method}({msg.params})")
                    
                    response = self._process_message(msg)
                    await websocket.send(json.dumps(response.to_dict()))
                    
                    print(f"[Backend] Sent response: {response.result}")
                    
            except Exception as e:
                print(f"[Backend] Client error: {e}")
            
            print("[Backend] Client disconnected")
        
        async def main():
            print(f"[Backend] Starting WebSocket server: ws://{host}:{port}")
            async with websockets.serve(handle_client, host, port):
                await asyncio.Future()  # Run forever
        
        asyncio.run(main())
    
    def stop(self):
        """Stop the server"""
        self.running = False


# Example: Custom business logic handlers
def handle_calculate(params: dict) -> dict:
    """Example: Calculator handler"""
    op = params.get("operation", "add")
    a = params.get("a", 0)
    b = params.get("b", 0)
    
    if op == "add":
        result = a + b
    elif op == "subtract":
        result = a - b
    elif op == "multiply":
        result = a * b
    elif op == "divide":
        result = a / b if b != 0 else float("inf")
    else:
        raise ValueError(f"Unknown operation: {op}")
    
    return {"result": result, "operation": op}


def handle_file_read(params: dict) -> dict:
    """Example: Read file content"""
    import os
    filepath = params.get("path", "")
    
    if not os.path.exists(filepath):
        raise FileNotFoundError(f"File not found: {filepath}")
    
    with open(filepath, "r", encoding="utf-8") as f:
        content = f.read()
    
    return {"path": filepath, "content": content, "size": len(content)}


def main():
    parser = argparse.ArgumentParser(description="ipckit Backend Server")
    parser.add_argument(
        "--mode", 
        choices=["pipe", "websocket"], 
        default="pipe",
        help="IPC mode: 'pipe' for desktop apps, 'websocket' for browsers"
    )
    parser.add_argument(
        "--name",
        default="ipckit_frontend",
        help="Channel name (for pipe mode)"
    )
    parser.add_argument(
        "--port",
        type=int,
        default=8765,
        help="WebSocket port (for websocket mode)"
    )
    
    args = parser.parse_args()
    
    # Create backend server
    backend = IpcBackend(channel_name=args.name)
    
    # Register custom handlers
    backend.register("calculate", handle_calculate)
    backend.register("file_read", handle_file_read)
    
    print("=" * 50)
    print("ipckit Frontend-Backend IPC Server")
    print("=" * 50)
    print(f"Mode: {args.mode}")
    print(f"Available methods: {list(backend.handlers.keys())}")
    print("=" * 50)
    
    try:
        if args.mode == "pipe":
            backend.run_pipe_server()
        else:
            backend.run_websocket_server(port=args.port)
    except KeyboardInterrupt:
        print("\n[Backend] Shutting down...")
        backend.stop()


if __name__ == "__main__":
    main()
