#!/usr/bin/env python3
"""Offline check of gates.py before the real run: a fake store, a fake origin, fake transcripts and a fake Fleet (two routes). No agent.
  python3 -I keeper_test.py <work-dir-outside-the-repo> <store-db-to-copy-the-schema-from>
Drives the fixes from run 48 and asserts each:
  1 independent gates   a merge is recorded while Checks is open (and logged as out of order)
  2 exact form          a shown file, a Markdown page and a document outside .armada/documents never close the document gate; an
                        HTML page under it closes only when its newest revision is approved
  3 stretches           the keeper writes no row for a `none` group; the live position is the first open gate; marks are the word only
  4 held, not silent    all_done with a gate open raises a Needs you item that says what is missing; the gate closing clears it
  5 mismatch            edits (agent and subagent) under a later group's paths while an earlier gate is open are logged
  Checks                a green run at a push that holds only one of two build groups does not close it; the next push does
  re-plan               a new version opens a new plan gate; gates whose group is unchanged carry over; Checks reopens when its stretch grew
Prints the keeper's events and ends with OK."""
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
tdir = f"{work}/home/user/.claude/projects/p/{sid}"
os.makedirs(f"{tdir}/subagents")

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

import atexit
atexit.register(lambda: open(f"{run_dir}/gates.stop", "w").close())  # the keeper ends with the test, pass or fail
JP = "apps/desktop/src/renderer/src/job-panes"
def ts(): return time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime()) + ".000Z"
def events(): return [json.loads(l) for l in open(f"{run_dir}/keeper.jsonl")] if os.path.exists(f"{run_dir}/keeper.jsonl") else []
def of(kind, **kw): return [e for e in events() if e["event"] == kind and all(e.get(k) == v for k, v in kw.items())]
def until(pred, what, secs=120):
    t = time.time()
    while time.time() - t < secs:
        if pred(): return
        time.sleep(0.3)
    raise SystemExit(f"timed out waiting for {what}\n" + "\n".join(json.dumps(e) for e in events()[-14:]))
def closed(gate): return bool(of("gate_closed", gate=gate))
def rows(): return [json.loads(l) for l in open(f"{run_dir}/gate_rows.jsonl")] if os.path.exists(f"{run_dir}/gate_rows.jsonl") else []
def st(): return json.load(open(f"{run_dir}/state.json"))

GROUPS = [
    ("Plan the change", "plan_approved", []),
    ("Write the design note", "document_approved", []),
    ("Store: drop one owner", "none", [f"{JP}/a/"]),
    ("Custom state by job and tab", "checks_green", [f"{JP}/b/"]),
    ("Deliver", "merged", []),
]
def record_plan(version, groups, approved=None):
    for t in ("session_plan", "session_plan_groups", "session_plan_tasks"):
        db.execute(f"delete from {t} where session_id=?", (sid,))
    db.execute("insert into session_plan(session_id,version,recorded_at,approved_at,changed,next_group,next_task,summary) values(?,?,?,?,0,0,0,'')", (sid, version, ts(), approved))
    for i, (t, g, paths) in enumerate(groups):
        db.execute("insert into session_plan_groups(session_id,position,group_id,agent,title,paths,gives) values(?,?,?,?,?,?,?)", (sid, i, f"g{i}", "x", t, json.dumps(paths), f"Something. [gate: {g}]"))
        db.execute("insert into session_plan_tasks(session_id,group_id,position,task_id,text,edited) values(?,?,?,?,?,0)", (sid, f"g{i}", i, f"t{i}", f"task {i}"))
    db.commit()
def mark_task(i):  # the agent's word: must change nothing about gates
    db.execute("update session_plan_tasks set started_at=?, completed_at=? where session_id=? and task_id=?", (ts(), ts(), sid, f"t{i}")); db.commit()
def artifact(target, revision=None, settled=None, form="document"):
    d = {"form": form}
    if form == "document":
        d.update({"asked": "true", "revision": str(revision), "revisions": json.dumps([f"r{k}" for k in range(1, revision + 1)])})
        if settled: d["settled"] = settled
    db.execute("insert into ledger_attachment_versions(holder_kind,holder_id,kind,manifest_id,target,state,detail,at) values('session',?, 'artifact','armada',?,'kept',?,?)", (sid, target, json.dumps(d), ts())); db.commit()
def transcript(path, edited, prompt=None):
    with open(path, "a") as f:
        if prompt:
            f.write(json.dumps({"timestamp": ts(), "type": "user", "message": {"content": prompt}}) + "\n")
        f.write(json.dumps({"timestamp": ts(), "type": "assistant", "message": {"content": [{"type": "tool_use", "name": "Edit", "input": {"file_path": edited}}]}}) + "\n")

record_plan(1, GROUPS)
until(lambda: of("plan_recorded"), "plan recorded")
assert of("plan_recorded")[0]["template_problems"] == [], of("plan_recorded")[0]["template_problems"]

# 5: edits under a later group's paths while the plan gate (and so every later gate) is open
transcript(f"{tdir}.jsonl", f"/x/slots/slot-1/{JP}/b/custom.ts")
transcript(f"{tdir}/subagents/agent-a1.jsonl", f"/x/slots/slot-1/{JP}/a/store.ts", prompt="Do group A only")
until(lambda: len(of("mismatch_edit")) >= 2, "two mismatch edits")
assert {e["by"] for e in of("mismatch_edit")} == {"agent", "sub:agent-a1"}, of("mismatch_edit")
until(lambda: of("subagent_seen"), "subagent seen")

# the plan gate
db.execute("update session_plan set approved_at=?", (ts(),)); db.commit()
until(lambda: closed("plan_approved"), "plan gate"); assert messages[-1].startswith("Fleet: the person approved the plan"), messages
until(lambda: os.path.exists(f"{run_dir}/state.json") and st()["live"] and st()["live"][1] == "Write the design note", "live moves to the note")

# 3: marks are the word only: marking every build task done changes no gate and writes no row
mark_task(2); mark_task(3); time.sleep(2.5)
assert [r["gate"] for r in rows()] == ["plan_approved"], rows()
assert st()["live"][1] == "Write the design note"
assert of("agent_word_group"), "the agent's word is logged"

# 4: all_done with the document gate open: held, and an item is raised that says what is missing
state["session"]["done_since"] = ts()
until(lambda: of("needs_you_raised"), "needs you item")
item = of("needs_you_raised")[0]
assert item["gate"] == "document_approved" and "no HTML page" in item["text"] and item["options"] == ["Approve", "Skip this group"], item
until(lambda: st()["needs_you"], "state carries the item")
assert st()["needs_you"][0]["id"] == item["id"]

# 2: a file shown, a Markdown page, and a document outside .armada/documents never close the gate
artifact("/x/.armada/documents/note.md", form="file")
artifact("/x/docs/note.html", 1, "approved")
artifact("/x/.armada/documents/note.md", 1, "approved")
time.sleep(2.5)
assert not closed("document_approved"), "wrong forms must not close it"
assert len(of("artifact_ignored")) >= 2, of("artifact_ignored")
# the real thing: an HTML page under it, sent back at revision 1, approved at revision 2
page = "/x/.armada/documents/design.html"
artifact(page, 1); artifact(page, 1, "sent_back"); time.sleep(2); assert not closed("document_approved"), "sent back is not approval"
artifact(page, 2); time.sleep(1.5); assert not closed("document_approved")
artifact(page, 2, "approved"); until(lambda: closed("document_approved"), "document gate")
assert messages[-1] == "Fleet: the person approved the design note.", messages
until(lambda: of("held_released"), "release"); until(lambda: of("needs_you_cleared", why="the gate closed"), "item cleared")
assert of("gate_closed", gate="document_approved")[0]["revision"] == "2"
until(lambda: st()["needs_you"] == [], "state clears the item")

# Checks: a first push that holds only group A's files runs green and does not close the gate
w = f"{work}/w"; subprocess.run(["git", "clone", "-q", f"{work}/origin.git", w], check=True)
g = lambda *a: subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", "-C", w, *a], check=True, capture_output=True)
g("checkout", "-q", "-b", f"armada/session-{id8}")
os.makedirs(f"{w}/{JP}/a", exist_ok=True); os.makedirs(f"{w}/{JP}/b", exist_ok=True)
open(f"{w}/{JP}/a/x.ts", "w").write("export const x = 1;\n")
open(f"{w}/{JP}/zz-keeper.test.ts", "w").write('import { expect, test } from "vitest";\ntest("keeper", () => expect(1).toBe(1));\n')
g("add", "-A"); g("commit", "-q", "-m", "group A"); g("push", "-q", "origin", "HEAD")
until(lambda: of("checks_green_but_stretch_incomplete"), "green but incomplete", 240)
assert not closed("checks_green") and of("checks_run")[0]["green"], events()[-6:]
assert of("checks_green_but_stretch_incomplete")[0]["lacking"] == ["Custom state by job and tab"]
assert st()["live"][1] == "Custom state by job and tab", st()["live"]

# 1: the merge is recorded while Checks is open, and logged as out of order
g("checkout", "-q", "main"); g("merge", "-q", "--no-ff", "-m", "merge", f"armada/session-{id8}"); g("push", "-q", "origin", "main")
until(lambda: closed("merged"), "merged gate recorded while Checks is open")
assert of("mismatch_gate_closed_out_of_order", gate="merged")[0]["earlier_open"] == ["Custom state by job and tab"], of("mismatch_gate_closed_out_of_order")
assert not closed("checks_green")

# the second push holds group B too: Checks closes at that commit
g("checkout", "-q", f"armada/session-{id8}")
open(f"{w}/{JP}/b/y.ts", "w").write("export const y = 2;\n")
g("add", "-A"); g("commit", "-q", "-m", "group B"); g("push", "-q", "origin", "HEAD")
until(lambda: closed("checks_green"), "checks gate", 240)
row = [r for r in rows() if r["gate"] == "checks_green"][0]
tip = subprocess.run(["git", "-C", w, "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()
assert row["sha"] == tip and row["stretch"] == ["Store: drop one owner", "Custom state by job and tab"], row
assert messages[-1].startswith("Fleet: Checks passed at "), messages

# re-plan: a new version opens a new plan gate; the document and merge gates carry; Checks reopens because its stretch grew
G2 = [GROUPS[0], GROUPS[1], GROUPS[2], ("Custom state by job and tab", "none", [f"{JP}/b/"]), ("Card names the tab", "checks_green", ["apps/desktop/src/renderer/src/mods-made.ts"]), GROUPS[4]]
record_plan(2, G2)
until(lambda: of("plan_gate_opened"), "plan v2")
ev = of("plan_gate_opened")[0]
print("carried closed:", ev["carried_closed"], "| open now:", ev["open_now"])
assert "Write the design note" in ev["carried_closed"] and "Deliver" in ev["carried_closed"], ev
assert "Card names the tab" in ev["open_now"] and "Plan the change" in ev["open_now"], ev

open(f"{run_dir}/gates.stop", "w").close(); keeper.wait(timeout=60)
for e in events():
    print(e["at"][11:23], e["event"], {k: v for k, v in e.items() if k not in ("at", "event")})
print("messages:", *messages, sep="\n  ")
assert not of("keeper_error"), of("keeper_error")
print("OK")
