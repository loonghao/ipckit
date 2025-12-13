"""
ipckit Frontend-Backend IPC Example - Python Frontend Client

This simulates a frontend client connecting to the backend server.
In real applications, this would be JavaScript code in a WebView.

Usage:
    # First start the backend server:
    python backend_server.py --mode pipe
    
    # Then run this client:
    python frontend_client.py
"""

import json
import sys
import time
import uuid

try:
    import ipckit
except ImportError:
    print("Error: ipckit not installed. Run: maturin develop --features python-bindings,ext-module")
    sys.exit(1)


class IpcClient:
    """Frontend IPC Client for communicating with Python backend"""
    
    def __init__(self, channel_name: str = "ipckit_frontend"):
        self.channel_name = channel_name
        self._channel = None
    
    def connect(self):
        """Connect to the backend server"""
        print(f"[Frontend] Connecting to backend: {self.channel_name}")
        self._channel = ipckit.IpcChannel.connect(self.channel_name)
        print(f"[Frontend] Connected!")
    
    def call(self, method: str, params: dict = None) -> dict:
        """
        Call an RPC method on the backend.
        
        Args:
            method: Method name to call
            params: Method parameters
            
        Returns:
            Response result or raises exception on error
        """
        if params is None:
            params = {}
        
        # Create request message
        request = {
            "id": str(uuid.uuid4()),
            "method": method,
            "params": params
        }
        
        # Send request
        self._channel.send_json(request)
        
        # Wait for response
        response = self._channel.recv_json()
        
        if response.get("error"):
            raise Exception(response["error"])
        
        return response.get("result")
    
    def close(self):
        """Close the connection"""
        self._channel = None


def main():
    client = IpcClient()
    
    try:
        # Connect to backend
        client.connect()
        
        print("\n" + "=" * 50)
        print("Testing IPC Communication")
        print("=" * 50)
        
        # Test 1: Ping
        print("\n[Test 1] Ping...")
        result = client.call("ping")
        print(f"  Result: {result}")
        
        # Test 2: Echo
        print("\n[Test 2] Echo...")
        result = client.call("echo", {"message": "Hello from frontend!"})
        print(f"  Result: {result}")
        
        # Test 3: Get backend info
        print("\n[Test 3] Get backend info...")
        result = client.call("get_info")
        print(f"  Result: {json.dumps(result, indent=2)}")
        
        # Test 4: Calculator
        print("\n[Test 4] Calculator...")
        result = client.call("calculate", {"operation": "multiply", "a": 7, "b": 6})
        print(f"  7 * 6 = {result['result']}")
        
        result = client.call("calculate", {"operation": "add", "a": 100, "b": 200})
        print(f"  100 + 200 = {result['result']}")
        
        # Test 5: Error handling
        print("\n[Test 5] Error handling (calling unknown method)...")
        try:
            result = client.call("unknown_method")
        except Exception as e:
            print(f"  Expected error: {e}")
        
        print("\n" + "=" * 50)
        print("All tests passed!")
        print("=" * 50)
        
    except Exception as e:
        print(f"Error: {e}")
    finally:
        client.close()


if __name__ == "__main__":
    main()
