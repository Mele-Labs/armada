#!/usr/bin/env python3
"""Offline check of gates.py before the real run: a fake store, a fake origin and a fake Fleet (two routes). No agent.
  python3 -I keeper_test.py <work-dir-outside-the-repo> <store-db-to-copy-the-schema-from>
Drives the same facts the real run will: plan approval, a document sent back and approved, none gates, a push, Checks, a hold at
all_done, the merge. Prints the keeper's events and asserts the order."""
import json, os, shutil, sqlite3, subprocess, sys, threading, time
from http.server import BaseHTTPRequestHandler, HTTPServer

work, schema_db = os.path.abspath(sys.argv[1]), sys.argv[2]
here = os.path.dirname(os.path.abspath(__file__))
rig = os.path.join(here, "..", "042-rig")
tree = subprocess.run(["git", "-C", here, "rev-parse", "--show-toplevel"], capture_output=True, text=True).stdout.strip()
shutil.rmtree(work, ignore_errors=True)
sid = "abcd1234-0000-0000-0000-000000000000"
id8 = sid[:8]
home = f"{work}/home/user/Library/Application Support/Armada"
os.makedirs(home)
run_dir = f"{work}/run"
os.makedirs(run_dir)

# fake store, schema copied from a real one
src = sqlite3.connect(f"file:{schema_db}?mode=ro", uri=True)
db = sqlite3.connect(f"{home}/armada.db")
for t in ("session_plan", "session_plan_groups", "session_plan_tasks", "session_done", "ledger_attachment_versions"):
    db.execute(src.execute("select sql from sqlite_master where name=?", (t,)).fetchone()[0])
db.commit()

# fake origin: main from this tree's current commit
subprocess.run(["git", "init", "--bare", "-q", f"{work}/origin.git"], check=True)
base = subprocess.run(["git", "-C", tree, "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()
subprocess.run(["git", "-C", tree, "push", "-q", f"{work}/origin.git", f"{base}:refs/heads/main"], check=True)

# fake Fleet
state = {"session": {"done_since": None, "hosted": {"turn": {"state": "idle"}}, "waiting_for": []}}
messages = []

class H(BaseHTTPRequestHandler):
    def log_message(self, *a): pass
    def do_GET(self):
        b = json.dumps(state).encode(); self.send_response(200); self.send_header("content-type", "application/json"); self.end_headers(); self.wfile.write(b)
    def do_POST(self):
        n = int(self.headers["content-length"]); messages.append(json.loads(self.rfile.read(n))["text"])
        self.send_response(200); self.send_header("content-type", "application/json"); self.end_headers(); self.wfile.write(b"{}")

srv = HTTPServer(("127.0.0.1", 0), H)
threading.Thread(target=srv.serve_forever, daemon=True).start()
json.dump({"port": srv.server_address[1], "pid": os.getpid()}, open(f"{home}/fleet.json", "w"))

keeper = subprocess.Popen([sys.executable, "-I", f"{here}/gates.py", "--scratch", work, "--session", sid, "--run-dir", run_dir, "--template", tree, "--rig", rig, "--poll", "0.5"])

def ts(): return time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime()) + ".000Z"
def events(): return [json.loads(l) for l in open(f"{run_dir}/keeper.jsonl")] if os.path.exists(f"{run_dir}/keeper.jsonl") else []
def until(pred, what, secs=120):
    t = time.time()
    while time.time() - t < secs:
        if pred(): return
        time.sleep(0.3)
    raise SystemExit(f"timed out waiting for {what}\n" + "\n".join(json.dumps(e) for e in events()[-12:]))
def closed(gate): return any(e["event"] == "gate_closed" and e["gate"] == gate for e in events())

TITLES = [("Plan the change", "plan_approved"), ("Write the design note", "document_approved"), ("Store: drop one owner", "none"), ("Rule and wiring", "none"), ("Tests and sweep", "checks_green"), ("Deliver", "merged")]
def record_plan(version, titles, approved=None):
    db.execute("delete from session_plan where session_id=?", (sid,)); db.execute("delete from session_plan_groups where session_id=?", (sid,)); db.execute("delete from session_plan_tasks where session_id=?", (sid,))
    db.execute("insert into session_plan(session_id,version,recorded_at,approved_at,changed,next_group,next_task,summary) values(?,?,?,?,0,0,0,'')", (sid, version, ts(), approved))
    for i, (t, g) in enumerate(titles):
        db.execute("insert into session_plan_groups(session_id,position,group_id,agent,title,paths,gives) values(?,?,?,?,?,?,?)", (sid, i, f"g{i}", "x", t, "[]", f"Something. [gate: {g}]"))
        db.execute("insert into session_plan_tasks(session_id,group_id,position,task_id,text,edited) values(?,?,?,?,?,0)", (sid, f"g{i}", i, f"t{i}", f"task {i}"))
    db.commit()
def start_task(i): db.execute("update session_plan_tasks set started_at=? where session_id=? and task_id=?", (ts(), sid, f"t{i}")); db.commit()
def doc(revision, settled=None):
    d = {"form": "document", "asked": "true", "revision": str(revision), "revisions": json.dumps([f"r{k}" for k in range(1, revision + 1)])}
    if settled: d["settled"] = settled
    db.execute("insert into ledger_attachment_versions(holder_kind,holder_id,kind,manifest_id,target,state,detail,at) values('session',?, 'artifact','armada','/x/.armada/documents/n.html','kept',?,?)", (sid, json.dumps(d), ts())); db.commit()

record_plan(1, TITLES)
until(lambda: any(e["event"] == "plan_recorded" for e in events()), "plan recorded")
assert [e for e in events() if e["event"] == "plan_recorded"][0]["template_problems"] == [], "template check should pass"
db.execute("update session_plan set approved_at=?", (ts(),)); db.commit()
until(lambda: closed("plan_approved"), "plan gate"); assert messages[-1].startswith("Fleet: the person approved the plan"), messages
doc(1); doc(1, "sent_back"); time.sleep(2); assert not closed("document_approved"), "sent back is not approval"
doc(2); time.sleep(1.5); assert not closed("document_approved")
doc(2, "approved"); until(lambda: closed("document_approved"), "document gate"); assert messages[-1] == "Fleet: the person approved the design note.", messages
start_task(3); until(lambda: any(e["event"] == "gate_closed" and e["title"] == "Store: drop one owner" for e in events()), "none gate 1")
# a push before the Checks group is due is only logged
w = f"{work}/w"; subprocess.run(["git", "clone", "-q", f"{work}/origin.git", w], check=True)
g = lambda *a: subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", "-C", w, *a], check=True, capture_output=True)
g("checkout", "-q", "-b", f"armada/session-{id8}")
os.makedirs(f"{w}/apps/desktop/src/renderer/src/job-panes", exist_ok=True)
open(f"{w}/apps/desktop/src/renderer/src/job-panes/zz-keeper.test.ts", "w").write('import { expect, test } from "vitest";\ntest("keeper", () => expect(1).toBe(1));\n')
g("add", "-A"); g("commit", "-q", "-m", "a test"); g("push", "-q", "origin", "HEAD")
until(lambda: any(e["event"] == "checks_not_due" for e in events()), "checks not due while 'Rule and wiring' is open")
assert not closed("checks_green")
start_task(4); until(lambda: closed("checks_green"), "checks gate", 240)
assert messages[-1].startswith("Fleet: Checks passed at "), messages
# all_done while the merge gate is open: held; the merge releases it with a notice
state["session"]["done_since"] = ts(); until(lambda: any(e["event"] == "held" for e in events()), "held at all_done")
assert [e for e in events() if e["event"] == "held"][0]["gate"] == "merged"
subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", "-C", w, "checkout", "-q", "main"], check=True)
g("merge", "-q", "--no-ff", "-m", "merge", f"armada/session-{id8}"); g("push", "-q", "origin", "main")
until(lambda: closed("merged"), "merged gate"); until(lambda: any(e["event"] == "held_released" for e in events()), "release")
assert messages[-1] == "Fleet: the branch is merged into main.", messages
# a re-plan: new version opens a new plan gate; titles that carried over stay closed
record_plan(2, TITLES + [("Custom tab state", "none")][:0] + [])
until(lambda: any(e["event"] == "plan_gate_opened" for e in events()), "plan v2")
ev = [e for e in events() if e["event"] == "plan_gate_opened"][0]
print("carried closed:", ev["carried_closed"], "| reopened:", ev["closed_before_but_open_now"])
assert "Write the design note" in ev["carried_closed"]
open(f"{run_dir}/gates.stop", "w").close(); keeper.wait(timeout=30)
for e in events():
    print(e["at"][11:23], e["event"], {k: v for k, v in e.items() if k not in ("at", "event")})
print("messages:", *messages, sep="\n  ")
print("OK")
