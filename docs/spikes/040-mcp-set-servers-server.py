#!/usr/bin/env python3
"""A stdio MCP server with one tool, `ping`, for the mcp-set-servers spike.

Answers `initialize`, `tools/list` and `tools/call`; ignores notifications.
Its name comes from argv[1], so one file serves every server the spike adds.
"""
import json
import sys

NAME = sys.argv[1] if len(sys.argv) > 1 else "spike"


def reply(msg_id, result):
    sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": msg_id, "result": result}) + "\n")
    sys.stdout.flush()


for line in sys.stdin:
    try:
        msg = json.loads(line)
    except ValueError:
        continue
    method, msg_id = msg.get("method"), msg.get("id")
    if msg_id is None:
        continue
    if method == "initialize":
        reply(msg_id, {
            "protocolVersion": msg.get("params", {}).get("protocolVersion", "2025-06-18"),
            "capabilities": {"tools": {}},
            "serverInfo": {"name": NAME, "version": "0.0.1"},
        })
    elif method == "tools/list":
        reply(msg_id, {"tools": [{
            "name": "ping",
            "description": "Answers pong.",
            "inputSchema": {"type": "object", "properties": {}},
        }]})
    elif method == "tools/call":
        reply(msg_id, {"content": [{"type": "text", "text": "pong from " + NAME}]})
    else:
        sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": msg_id,
                                     "error": {"code": -32601, "message": "no such method"}}) + "\n")
        sys.stdout.flush()
