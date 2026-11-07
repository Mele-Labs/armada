#!/usr/bin/env python3
"""Probe: can a task list be seeded into a running claude, and changed from outside?

  python3 -I probe.py <scratch-home> <cwd> <list-id>

Starts one haiku `claude -p` in stream-json mode (as Fleet hosts it) under the scratch HOME.
  1. the process starts with NO list files
  2. this script writes tasks 1..4 into ~/.claude/tasks/<list>/ (seeded after start, before turn one)
  3. turn one:  "call TaskList and TaskGet 2, print what you saw"            (a)
  4. this script marks task 1 completed in its file
  5. turn two:  "call TaskList again, then TaskGet 1"                        (b)
  6. turn three: "TaskCreate 'agent own', then TaskList"                     (c)
Prints each tool call and result.
"""
import json, os, subprocess, sys, time

home, cwd, lst = sys.argv[1:4]
d = f"{home}/.claude/tasks/{lst}"
env = dict(os.environ, HOME=home, CLAUDE_CODE_TASK_LIST_ID=lst)
p = subprocess.Popen(
    ["claude", "-p", "--model", "haiku", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose",
     "--permission-mode", "default"],
    cwd=cwd, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)


def seed():
    os.makedirs(d, exist_ok=True)
    for i, (s, blocked) in enumerate([("Step: find the cause", []), ("Gate: the person approves the plan. Fleet ticks this", ["1"]),
                                       ("Step: fix", ["2"]), ("Gate: Checks green at the pushed commit. Fleet ticks this", ["3"])], 1):
        json.dump({"id": str(i), "subject": s, "description": f"description of task {i}", "activeForm": f"doing {i}",
                   "status": "pending", "blocks": [], "blockedBy": blocked}, open(f"{d}/{i}.json", "w"), indent=2)


def turn(text):
    p.stdin.write(json.dumps({"type": "user", "message": {"role": "user", "content": text}}) + "\n")
    p.stdin.flush()
    for line in p.stdout:
        o = json.loads(line)
        if o.get("type") == "assistant":
            for c in o["message"]["content"]:
                if c["type"] == "tool_use":
                    print("  TOOL", c["name"], json.dumps(c["input"])[:160])
                elif c["type"] == "text":
                    print("  TEXT", c["text"][:400].replace("\n", " | "))
        elif o.get("type") == "user":
            for c in o["message"]["content"]:
                if isinstance(c, dict) and c.get("type") == "tool_result":
                    print("  RESULT", str(c.get("content"))[:500].replace("\n", " | "))
        elif o.get("type") == "result":
            return


time.sleep(4)  # the process is up with no list
seed()
print("(a) seeded 4 tasks after the process started, before turn one")
turn("Load TaskList and TaskGet with ToolSearch if needed. Call TaskList, then TaskGet on task 2. Print what you saw, verbatim, in one reply.")
f = json.load(open(f"{d}/1.json")); f["status"] = "completed"; json.dump(f, open(f"{d}/1.json", "w"), indent=2)
print("(b) Fleet marked task 1 completed on disk")
turn("Call TaskList again, then TaskGet on task 1. Print status of every task verbatim.")
print("(c) the agent makes its own")
turn("Call TaskCreate with subject 'agent own', description 'x', activeForm 'x'. Then call TaskList. Print it.")
p.stdin.close(); p.wait(timeout=30)
print("files now:", sorted(os.listdir(d)))
for n in sorted(os.listdir(d)):
    if n.endswith(".json"):
        j = json.load(open(f"{d}/{n}")); print(" ", n, j["status"], j["subject"], "blockedBy", j["blockedBy"])
