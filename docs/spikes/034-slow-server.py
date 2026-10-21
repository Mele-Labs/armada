#!/usr/bin/env python3
"""A stdio MCP server that sleeps DELAY seconds before answering `initialize`.
argv: name delay. Exposes one tool, `ask_the_person`, so it can stand in for the door."""
import json, sys, time
name, delay = sys.argv[1], float(sys.argv[2])
def send(o): sys.stdout.write(json.dumps(o) + "\n"); sys.stdout.flush()
for line in sys.stdin:
    try: m = json.loads(line)
    except Exception: continue
    mid, meth = m.get("id"), m.get("method")
    if meth == "initialize":
        time.sleep(delay)
        send({"jsonrpc": "2.0", "id": mid, "result": {"protocolVersion": m["params"]["protocolVersion"], "capabilities": {"tools": {}}, "serverInfo": {"name": name, "version": "1"}}})
    elif meth == "tools/list":
        send({"jsonrpc": "2.0", "id": mid, "result": {"tools": [{"name": "ask_the_person", "description": "approve", "inputSchema": {"type": "object", "properties": {"tool_name": {"type": "string"}, "input": {"type": "object"}}}}]}})
    elif meth == "tools/call":
        a = m["params"]["arguments"]
        send({"jsonrpc": "2.0", "id": mid, "result": {"content": [{"type": "text", "text": json.dumps({"behavior": "allow", "updatedInput": a.get("input", {})})}]}})
    elif mid is not None:
        send({"jsonrpc": "2.0", "id": mid, "result": {}})
