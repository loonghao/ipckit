"""
ipckit Desktop WebView Example

This example shows how to use ipckit with a desktop WebView application.
The WebView (using pywebview) communicates with the Python backend via Named Pipe.

Requirements:
    pip install pywebview

Usage:
    python desktop_webview.py
"""

import json
import sys
import threading
import time
import uuid
from typing import Any, Callable, Dict

try:
    import ipckit
except ImportError:
    print("Error: ipckit not installed. Run: maturin develop --features python-bindings,ext-module")
    sys.exit(1)

try:
    import webview
except ImportError:
    print("Error: pywebview is required. Install with: pip install pywebview")
    sys.exit(1)


class IpcBridge:
    """
    Bridge between WebView JavaScript and Python backend via IPC.
    
    This class is exposed to JavaScript and provides RPC-style communication.
    """
    
    def __init__(self):
        self.handlers: Dict[str, Callable] = {}
        self._setup_handlers()
    
    def _setup_handlers(self):
        """Register built-in handlers"""
        self.handlers["ping"] = lambda p: {"pong": True, "timestamp": time.time()}
        self.handlers["echo"] = lambda p: p.get("message", "")
        self.handlers["get_info"] = lambda p: {
            "version": getattr(ipckit, "__version__", "0.1.0"),
            "python_version": sys.version,
            "platform": sys.platform,
        }
        self.handlers["calculate"] = self._calculate
        self.handlers["file_read"] = self._file_read
    
    def _calculate(self, params: dict) -> dict:
        op = params.get("operation", "add")
        a = params.get("a", 0)
        b = params.get("b", 0)
        
        ops = {
            "add": lambda: a + b,
            "subtract": lambda: a - b,
            "multiply": lambda: a * b,
            "divide": lambda: a / b if b != 0 else float("inf"),
        }
        
        result = ops.get(op, lambda: 0)()
        return {"result": result, "operation": op}
    
    def _file_read(self, params: dict) -> dict:
        import os
        filepath = params.get("path", "")
        
        if not os.path.exists(filepath):
            raise FileNotFoundError(f"File not found: {filepath}")
        
        with open(filepath, "r", encoding="utf-8") as f:
            content = f.read()
        
        return {"path": filepath, "content": content, "size": len(content)}
    
    def call(self, method: str, params_json: str = "{}") -> str:
        """
        Call a Python method from JavaScript.
        
        This method is exposed to JavaScript via pywebview's JS API.
        
        Args:
            method: Method name to call
            params_json: JSON-encoded parameters
            
        Returns:
            JSON-encoded response
        """
        try:
            params = json.loads(params_json)
        except json.JSONDecodeError:
            params = {}
        
        handler = self.handlers.get(method)
        
        if handler is None:
            return json.dumps({
                "error": f"Unknown method: {method}",
                "result": None
            })
        
        try:
            result = handler(params)
            return json.dumps({"result": result, "error": None})
        except Exception as e:
            return json.dumps({"result": None, "error": str(e)})
    
    def get_methods(self) -> str:
        """Get list of available methods (for JavaScript)"""
        return json.dumps(list(self.handlers.keys()))


# HTML content for the WebView
HTML_CONTENT = """
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>ipckit Desktop App</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            color: #fff;
            padding: 20px;
        }
        
        .container { max-width: 600px; margin: 0 auto; }
        
        h1 {
            text-align: center;
            margin-bottom: 30px;
            font-size: 2em;
        }
        
        .card {
            background: rgba(255,255,255,0.15);
            backdrop-filter: blur(10px);
            border-radius: 15px;
            padding: 20px;
            margin-bottom: 20px;
        }
        
        .card h2 {
            margin-bottom: 15px;
            font-size: 1.2em;
        }
        
        .btn-group { display: flex; flex-wrap: wrap; gap: 10px; }
        
        button {
            padding: 12px 24px;
            border: none;
            border-radius: 8px;
            font-size: 14px;
            font-weight: 600;
            cursor: pointer;
            background: rgba(255,255,255,0.2);
            color: #fff;
            transition: all 0.3s;
        }
        
        button:hover {
            background: rgba(255,255,255,0.3);
            transform: translateY(-2px);
        }
        
        .calculator {
            display: grid;
            grid-template-columns: 1fr auto 1fr auto auto;
            gap: 10px;
            align-items: center;
        }
        
        input, select {
            padding: 12px;
            border: none;
            border-radius: 8px;
            background: rgba(255,255,255,0.2);
            color: #fff;
            font-size: 16px;
        }
        
        input::placeholder { color: rgba(255,255,255,0.5); }
        
        .result {
            font-size: 1.5em;
            font-weight: bold;
            min-width: 80px;
            text-align: center;
        }
        
        .log {
            background: rgba(0,0,0,0.3);
            border-radius: 10px;
            padding: 15px;
            height: 200px;
            overflow-y: auto;
            font-family: monospace;
            font-size: 12px;
        }
        
        .log-entry { padding: 3px 0; }
        .log-entry.request { color: #ffd700; }
        .log-entry.response { color: #90ee90; }
        .log-entry.error { color: #ff6b6b; }
    </style>
</head>
<body>
    <div class="container">
        <h1>🖥️ ipckit Desktop Demo</h1>
        
        <div class="card">
            <h2>📡 IPC Methods</h2>
            <div class="btn-group">
                <button onclick="callMethod('ping')">Ping</button>
                <button onclick="callMethod('echo', {message: 'Hello!'})">Echo</button>
                <button onclick="callMethod('get_info')">Get Info</button>
            </div>
        </div>
        
        <div class="card">
            <h2>🧮 Calculator</h2>
            <div class="calculator">
                <input type="number" id="numA" value="10">
                <select id="op">
                    <option value="add">+</option>
                    <option value="subtract">-</option>
                    <option value="multiply">×</option>
                    <option value="divide">÷</option>
                </select>
                <input type="number" id="numB" value="5">
                <button onclick="calculate()">=</button>
                <span class="result" id="result">?</span>
            </div>
        </div>
        
        <div class="card">
            <h2>📋 Log</h2>
            <div class="log" id="log"></div>
        </div>
    </div>

    <script>
        function log(type, msg) {
            const el = document.getElementById('log');
            const entry = document.createElement('div');
            entry.className = 'log-entry ' + type;
            entry.textContent = `[${new Date().toLocaleTimeString()}] ${msg}`;
            el.appendChild(entry);
            el.scrollTop = el.scrollHeight;
        }
        
        async function callMethod(method, params = {}) {
            log('request', `→ ${method}(${JSON.stringify(params)})`);
            
            try {
                const result = await window.pywebview.api.call(method, JSON.stringify(params));
                const response = JSON.parse(result);
                
                if (response.error) {
                    log('error', `✗ ${response.error}`);
                } else {
                    log('response', `← ${JSON.stringify(response.result)}`);
                }
                
                return response.result;
            } catch (err) {
                log('error', `✗ ${err.message}`);
            }
        }
        
        async function calculate() {
            const a = parseFloat(document.getElementById('numA').value);
            const b = parseFloat(document.getElementById('numB').value);
            const op = document.getElementById('op').value;
            
            const result = await callMethod('calculate', { operation: op, a, b });
            if (result) {
                document.getElementById('result').textContent = result.result;
            }
        }
        
        // Wait for pywebview to be ready
        window.addEventListener('pywebviewready', () => {
            log('response', '✓ pywebview ready - IPC bridge active');
        });
    </script>
</body>
</html>
"""


def main():
    print("=" * 50)
    print("ipckit Desktop WebView Demo")
    print("=" * 50)
    
    # Create IPC bridge
    bridge = IpcBridge()
    
    # Create WebView window
    window = webview.create_window(
        title="ipckit Desktop Demo",
        html=HTML_CONTENT,
        width=700,
        height=600,
        js_api=bridge,  # Expose bridge to JavaScript
    )
    
    print("Starting WebView...")
    print("Available IPC methods:", list(bridge.handlers.keys()))
    
    # Start WebView (blocking)
    webview.start()
    
    print("Application closed.")


if __name__ == "__main__":
    main()
