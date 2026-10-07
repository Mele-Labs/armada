#!/usr/bin/env python3
"""Records every hook POST and answers by HOOK_MODE: allow | deny."""
import json, os, sys
from http.server import BaseHTTPRequestHandler, HTTPServer

LOG = os.environ.get("HOOK_LOG", "hook.log")
MODE = os.environ.get("HOOK_MODE", "allow")


class H(BaseHTTPRequestHandler):
    def do_POST(self):
        n = int(self.headers.get("content-length", "0"))
        body = self.rfile.read(n).decode()
        with open(LOG, "a") as f:
            f.write(json.dumps({"path": self.path, "body": json.loads(body) if body else None}) + "\n")
        out = {}
        if MODE == "deny":
            out = {"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny",
                   "permissionDecisionReason": "Held until leased"}}
        data = json.dumps(out).encode()
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def log_message(self, *a):
        pass


HTTPServer(("127.0.0.1", int(sys.argv[1])), H).serve_forever()
