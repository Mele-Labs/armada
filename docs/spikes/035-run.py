#!/usr/bin/env python3
"""One spike run. argv: label [NAME=VALUE ...] [--loose]. Starts the CLI as a hosted job
is started, sends one turn, asks mcp_status at 60s, prints one JSON summary line.
Arguments: DOOR=<seconds> delays the door; --no-ppt drops --permission-prompt-tool; --loose drops
--strict-mcp-config. Run it from a scratch directory: the config and each line log land there."""
import json, os, subprocess, sys, threading, time
here = os.path.dirname(os.path.abspath(__file__))
label, rest = sys.argv[1], sys.argv[2:]
loose = "--loose" in rest
env = dict(os.environ)
for kv in rest:
    if "=" in kv:
        k, v = kv.split("=", 1); env[k] = v
door_delay = env.pop("DOOR", "0")
for k in ("MCP_TIMEOUT", "CLAUDE_CODE_MCP_STARTUP_WAIT_MS"):
    if k not in [kv.split("=")[0] for kv in rest]: env.pop(k, None)
cfg = os.path.join(os.getcwd(), "mcp.json")
json.dump({"mcpServers": {
    "armada-fleet": {"command": "python3", "args": [os.path.join(here, "034-slow-server.py"), "door", door_delay]},
    "slow": {"command": "python3", "args": [os.path.join(here, "034-slow-server.py"), "slow", "45"]},
}}, open(cfg, "w"))
argv = ["taskpolicy", "-c", "utility", "claude", "-p", "--input-format", "stream-json", "--output-format", "stream-json",
        "--verbose", "--model", "haiku", "--no-session-persistence",
        "--mcp-config", cfg]
if "--no-ppt" not in rest: argv += ["--permission-prompt-tool", "mcp__armada-fleet__ask_the_person"]
if not loose: argv.append("--strict-mcp-config")
p = subprocess.Popen(argv, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, env=env, text=True, cwd=os.getcwd())
t0 = time.monotonic()
out = {"label": label, "env": [kv for kv in rest if "=" in kv], "loose": loose, "ppt": "--no-ppt" not in rest}
log = open(os.path.join(os.getcwd(), f"{label}.ndjson"), "w")
def reader():
    for line in p.stdout:
        at = round(time.monotonic() - t0, 2)
        log.write(json.dumps({"at": at, "line": line.rstrip()}) + "\n"); log.flush()
        try: m = json.loads(line)
        except Exception: continue
        ty, sub = m.get("type"), m.get("subtype")
        if ty == "system" and sub == "init" and "init_at" not in out:
            out["init_at"] = at; out["init_servers"] = {s["name"]: s["status"] for s in m.get("mcp_servers", [])}
        elif ty == "assistant" and "first_assistant_at" not in out:
            out["first_assistant_at"] = at
        elif ty == "result" and "result_at" not in out:
            out["result_at"] = at
        elif ty == "control_response":
            r = m.get("response", {}).get("response", {})
            out["status_at"] = at; out["status_servers"] = {s["name"]: s["status"] for s in r.get("mcpServers", [])} or r
threading.Thread(target=reader, daemon=True).start()
p.stdin.write(json.dumps({"type": "user", "message": {"role": "user", "content": [{"type": "text", "text": "say ok"}]}}) + "\n"); p.stdin.flush()
time.sleep(60)
p.stdin.write(json.dumps({"type": "control_request", "request_id": "s1", "request": {"subtype": "mcp_status"}}) + "\n"); p.stdin.flush()
deadline = time.monotonic() + 30
while "status_at" not in out and time.monotonic() < deadline: time.sleep(0.5)
p.stdin.close()
try: p.wait(20)
except subprocess.TimeoutExpired: p.kill()
print(json.dumps(out), flush=True)
