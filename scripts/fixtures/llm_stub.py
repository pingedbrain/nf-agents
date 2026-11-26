#!/usr/bin/env python3
"""Offline stand-in for an OpenAI-compatible chat endpoint (stdlib, Python >= 3.9).

Usage: llm_stub.py PORT  -- serves on 127.0.0.1 and prints "ready" once listening.
POST .../chat/completions (non-streaming) gets a fixed text, or JSON valid for the request's
`json_schema`; anything else gets 501 naming the path. Nothing is logged or forwarded.
"""

import json
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

TEXT = "stub-ok"
SCALARS = {"string": TEXT, "number": 0.5, "integer": 1, "boolean": True}


def sample(schema):
    """A value valid for the JSON schema subset structured outputs use."""
    if not isinstance(schema, dict):
        return None
    if schema.get("enum"):
        return schema["enum"][0]
    t = schema.get("type")
    if t == "object":
        return {k: sample(v) for k, v in (schema.get("properties") or {}).items()}
    if t == "array":
        return [sample(schema.get("items"))]
    return SCALARS.get(t)


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, *args):
        pass

    def do_GET(self):
        self._send(501, {"error": {"message": f"llm stub: {self.path} not emulated"}})

    def do_POST(self):
        try:
            req = json.loads(self.rfile.read(int(self.headers.get("content-length") or 0)) or b"{}")
        except ValueError:
            return self._send(400, {"error": {"message": "llm stub: body is not JSON"}})
        if not self.path.endswith("/chat/completions") or not isinstance(req, dict) or req.get("stream"):
            return self._send(501, {"error": {"message": f"llm stub: {self.path} not emulated"}})
        fmt = req.get("response_format") or {}
        content = TEXT
        if isinstance(fmt, dict) and fmt.get("type") == "json_schema":
            content = json.dumps(sample((fmt.get("json_schema") or {}).get("schema")))
        self._send(200, {"id": "stub", "object": "chat.completion", "created": 0, "model": req.get("model"),
                         "choices": [{"index": 0, "finish_reason": "stop",
                                      "message": {"role": "assistant", "content": content}}],
                         "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}})

    def _send(self, code, obj):
        data = json.dumps(obj).encode()
        self.send_response(code)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)


def server(port):
    return ThreadingHTTPServer(("127.0.0.1", port), Handler)


if __name__ == "__main__":
    srv = server(int(sys.argv[1]))
    print("ready", flush=True)
    srv.serve_forever()
