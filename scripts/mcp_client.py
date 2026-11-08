#!/usr/bin/env python3
"""Development probe for the real Rust stdio MCP binary. Uses only stdlib."""

import argparse
import json
import queue
import subprocess
import sys
import threading
import time


class McpClient:
    def __init__(self, command, timeout=20):
        self.timeout = timeout
        self.counter = 0
        self.messages = queue.Queue()
        self.process = subprocess.Popen(
            command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=sys.stderr, text=True, bufsize=1,
        )
        self.reader = threading.Thread(target=self._read, daemon=True)
        self.reader.start()
        self.initialize = self.request("initialize", {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {"name": "fender-live-probe", "version": "0.1.0"},
        })
        self.notify("notifications/initialized", {})

    def _read(self):
        for line in self.process.stdout:
            try:
                self.messages.put(json.loads(line))
            except json.JSONDecodeError:
                self.messages.put({"probe_error": "non-JSON stdout", "line": line})
        self.messages.put({"probe_error": "server stdout closed"})

    def _send(self, payload):
        self.process.stdin.write(json.dumps(payload, ensure_ascii=False) + "\n")
        self.process.stdin.flush()

    def notify(self, method, params):
        self._send({"jsonrpc": "2.0", "method": method, "params": params})

    def request(self, method, params=None):
        self.counter += 1
        request_id = self.counter
        self._send({"jsonrpc": "2.0", "id": request_id, "method": method, "params": params or {}})
        deadline = time.monotonic() + self.timeout
        while time.monotonic() < deadline:
            remaining = max(0.001, deadline - time.monotonic())
            message = self.messages.get(timeout=remaining)
            if "probe_error" in message:
                raise RuntimeError(message)
            if message.get("id") == request_id:
                if "error" in message:
                    raise RuntimeError(message["error"])
                return message.get("result")
            if "method" in message and "id" in message:
                self._send({
                    "jsonrpc": "2.0", "id": message["id"],
                    "error": {"code": -32601, "message": "Probe client does not support this request"},
                })
        raise TimeoutError(f"MCP request {request_id} timed out")

    def call(self, name, arguments=None):
        return self.request("tools/call", {"name": name, "arguments": arguments or {}})

    def close(self):
        if self.process.poll() is None:
            self.process.stdin.close()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.terminate()
                self.process.wait(timeout=5)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--timeout", type=float, default=20)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command
    if command and command[0] == "--":
        command = command[1:]
    if not command:
        command = ["target/debug/fender-studio-pro-mcp", "serve"]
    client = McpClient(command, timeout=args.timeout)
    print(json.dumps({"initialized": client.initialize}), flush=True)
    try:
        for line in sys.stdin:
            if not line.strip():
                continue
            payload = json.loads(line)
            result = client.request(payload["method"], payload.get("params"))
            print(json.dumps(result, ensure_ascii=False), flush=True)
    finally:
        client.close()


if __name__ == "__main__":
    main()
