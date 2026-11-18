#!/usr/bin/env python3
"""Drive one headless `claude` the way Fleet hosts a job, and ask it to load MCP
servers while it runs. Writes every line it sends and hears to the transcript.

usage: mcp-set-servers-driver.py <workdir> <transcript.ndjson> [live|operator|unapproved]

`live` (run 1) adds servers by set and by `claude mcp add`. `operator` (run 2)
starts with a project server in `.mcp.json` and checks what a set leaves alone.
No user message is ever sent, so no turn runs and no model is called.
"""
import json
import os
import queue
import subprocess
import sys
import threading
import time

HERE = os.path.dirname(os.path.abspath(__file__))
SERVER = os.path.join(HERE, "mcp-set-servers-server.py")
WORK, TRANSCRIPT = sys.argv[1], sys.argv[2]
RUN = sys.argv[3] if len(sys.argv) > 3 else "live"
os.makedirs(WORK, exist_ok=True)


def stdio(name):
    return {"type": "stdio", "command": sys.executable, "args": [SERVER, name]}


door = os.path.join(WORK, "door.json")
with open(door, "w") as f:
    json.dump({"mcpServers": {"spike-door": stdio("spike-door")}}, f)

env = {k: v for k, v in os.environ.items() if not k.startswith(("CLAUDECODE", "CLAUDE_CODE_", "ARMADA_"))}
proc = subprocess.Popen(
    ["claude", "-p", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose",
     "--mcp-config", door, "--setting-sources", "local" if RUN == "live" else "project,local"]
    + ([] if RUN == "unapproved" else ["--settings", json.dumps({"enableAllProjectMcpServers": True})]),
    cwd=WORK, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
)
log = open(TRANSCRIPT, "w")
heard = queue.Queue()


def pump(stream, tag):
    for line in stream:
        heard.put((tag, line.rstrip("\n")))


threading.Thread(target=pump, args=(proc.stdout, "out"), daemon=True).start()
threading.Thread(target=pump, args=(proc.stderr, "err"), daemon=True).start()
counter = [0]


def ask(request, within=30):
    counter[0] += 1
    rid = "spike-%d" % counter[0]
    line = json.dumps({"type": "control_request", "request_id": rid, "request": request})
    log.write(json.dumps({"sent": json.loads(line)}) + "\n")
    proc.stdin.write(line + "\n")
    proc.stdin.flush()
    deadline = time.time() + within
    while time.time() < deadline:
        try:
            tag, text = heard.get(timeout=0.5)
        except queue.Empty:
            continue
        log.write(json.dumps({"heard": tag, "line": text}) + "\n")
        log.flush()
        if tag == "out" and rid in text and "control_response" in text:
            return json.loads(text)["response"]
    return {"timeout": within}


def names(status):
    servers = (status.get("response") or {}).get("mcpServers") or []
    return [(s["name"], s.get("status"), s.get("scope"), len(s.get("tools") or [])) for s in servers]


def step(title, request, within=30):
    answer = ask(request, within)
    print("==", title)
    if request["subtype"] == "mcp_status":
        print("  ", [n for n in names(answer) if n[0].startswith("spike")], "| all:", len(names(answer)))
    else:
        print("  ", json.dumps(answer)[:600])
    return answer


if RUN == "unapproved":
    # Run 3: the same .mcp.json, with nobody having approved its servers. Does a headless start load them?
    time.sleep(6)
    step("1 status at start", {"subtype": "mcp_status"})
    proc.stdin.close()
    proc.wait(timeout=15)
    log.close()
    sys.exit(0)

if RUN == "operator":
    # Run 2: a project server in .mcp.json from the start. Does a set touch servers from config files?
    time.sleep(4)
    step("1 status at start", {"subtype": "mcp_status"})
    step("2 set {spike-added}", {"subtype": "mcp_set_servers", "servers": {"spike-added": stdio("spike-added")}}, 60)
    step("3 status", {"subtype": "mcp_status"})
    step("4 set {} (empty)", {"subtype": "mcp_set_servers", "servers": {}}, 60)
    step("5 status after empty set", {"subtype": "mcp_status"})
    step("6 set {spike-file} (a name .mcp.json already holds)",
         {"subtype": "mcp_set_servers", "servers": {"spike-file": stdio("spike-file")}}, 60)
    step("7 status", {"subtype": "mcp_status"})
    step("8 set {spike-door} (the --mcp-config name)",
         {"subtype": "mcp_set_servers", "servers": {"spike-door": stdio("spike-door")}}, 60)
    step("9 set {} (empty)", {"subtype": "mcp_set_servers", "servers": {}}, 60)
    step("10 status", {"subtype": "mcp_status"})
    proc.stdin.close()
    proc.wait(timeout=15)
    while not heard.empty():
        tag, text = heard.get()
        log.write(json.dumps({"heard": tag, "line": text}) + "\n")
    log.close()
    sys.exit(0)

time.sleep(4)
step("1 status at start", {"subtype": "mcp_status"})
step("2 set {spike-added}", {"subtype": "mcp_set_servers", "servers": {"spike-added": stdio("spike-added")}}, 60)
time.sleep(2)
step("3 status after set", {"subtype": "mcp_status"})
step("4 set again, same entry", {"subtype": "mcp_set_servers", "servers": {"spike-added": stdio("spike-added")}}, 60)
step("5 set {spike-added, spike-door}",
     {"subtype": "mcp_set_servers", "servers": {"spike-added": stdio("spike-added"), "spike-door": stdio("spike-door")}}, 60)
step("6 status", {"subtype": "mcp_status"})

# A server added the way a job adds one: `claude mcp add`, project scope (.mcp.json in cwd).
add = subprocess.run(["claude", "mcp", "add", "-s", "project", "spike-file", "--", sys.executable, SERVER, "spike-file"],
                     cwd=WORK, env=env, capture_output=True, text=True)
print("== claude mcp add:", add.returncode, add.stdout.strip()[:200], add.stderr.strip()[:200])
time.sleep(3)
step("7 status after claude mcp add, nothing pushed", {"subtype": "mcp_status"})
step("8 set {spike-added, spike-file}",
     {"subtype": "mcp_set_servers", "servers": {"spike-added": stdio("spike-added"), "spike-file": stdio("spike-file")}}, 60)
time.sleep(2)
step("9 status", {"subtype": "mcp_status"})
step("10 set {} (empty)", {"subtype": "mcp_set_servers", "servers": {}}, 60)
step("11 status after empty set", {"subtype": "mcp_status"})

proc.stdin.close()
try:
    proc.wait(timeout=15)
except subprocess.TimeoutExpired:
    proc.terminate()
while not heard.empty():
    tag, text = heard.get()
    log.write(json.dumps({"heard": tag, "line": text}) + "\n")
log.close()
