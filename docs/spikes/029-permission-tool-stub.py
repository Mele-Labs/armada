#!/usr/bin/env python3
"""A stand-in --permission-prompt-tool server (spike 29). Logs each call it is asked
about and allows it; for AskUserQuestion it fills `answers` the way Fleet does:
labels joined with ", " for a multi-select, a fixed string otherwise."""
import json, sys, os
LOG=os.environ.get("STUB_LOG","stub.log")
def send(o): sys.stdout.write(json.dumps(o)+"\n"); sys.stdout.flush()
for line in sys.stdin:
    try: m=json.loads(line)
    except: continue
    mid=m.get("id"); meth=m.get("method")
    if meth=="initialize":
        send({"jsonrpc":"2.0","id":mid,"result":{"protocolVersion":m["params"]["protocolVersion"],"capabilities":{"tools":{}},"serverInfo":{"name":"stub","version":"1"}}})
    elif meth=="tools/list":
        send({"jsonrpc":"2.0","id":mid,"result":{"tools":[{"name":"approve","description":"approve","inputSchema":{"type":"object","properties":{"tool_name":{"type":"string"},"input":{"type":"object"},"tool_use_id":{"type":"string"}}}}]}})
    elif meth=="tools/call":
        a=m["params"]["arguments"]
        open(LOG,"a").write(json.dumps(a)+"\n")
        inp=a.get("input",{})
        if a.get("tool_name")=="AskUserQuestion":
            inp=dict(inp); inp["answers"]={q["question"]:", ".join(o["label"] for o in q["options"]) if q.get("multiSelect") else "my own words" for q in inp["questions"]}
        send({"jsonrpc":"2.0","id":mid,"result":{"content":[{"type":"text","text":json.dumps({"behavior":"allow","updatedInput":inp})}]}})
