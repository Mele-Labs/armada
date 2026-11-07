#!/usr/bin/env python3
"""Offline check of person.py's fixes before the real run, against a fake Fleet. No agent.
  python3 -I person_test.py <work-dir-outside-the-repo>
Asserts: (a) a merge asked while a gate before Deliver is open is not done, and the answer says what is open;
(b) a held item on the document gate with no open page is answered with what is missing, not approved;
(c) once merge_ready is true the merge happens and the origin's main moves;
(d) the quiet rule fires with a `done` item present (run 48 counted it and never stopped)."""
import json, os, shutil, subprocess, sys, threading, time
from http.server import BaseHTTPRequestHandler, HTTPServer

work = os.path.abspath(sys.argv[1])
here = os.path.dirname(os.path.abspath(__file__))
shutil.rmtree(work, ignore_errors=True)
sid = "abcd1234-0000-0000-0000-000000000000"
id8 = sid[:8]
home = f"{work}/home/user/Library/Application Support/Armada"
run_dir = f"{work}/run"
os.makedirs(home); os.makedirs(run_dir)
# an origin with main and the Job's branch one commit ahead
subprocess.run(["git", "init", "--bare", "-q", f"{work}/origin.git"], check=True)
w = f"{work}/w"
subprocess.run(["git", "init", "-q", "-b", "main", w], check=True)
g = lambda *a: subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", "-C", w, *a], check=True, capture_output=True)
open(f"{w}/a", "w").write("1"); g("add", "-A"); g("commit", "-q", "-m", "base"); g("push", "-q", f"{work}/origin.git", "main")
g("checkout", "-q", "-b", f"armada/session-{id8}"); open(f"{w}/b", "w").write("2"); g("add", "-A"); g("commit", "-q", "-m", "work"); g("push", "-q", f"{work}/origin.git", f"armada/session-{id8}")

state = {"session": {"state": "live", "done_since": "2026-11-07T00:00:00.000Z", "hosted": {"turn": {"state": "idle"}}, "waiting_for": [
    {"id": "done", "source": "done", "text": "done", "since": "x"},
    {"id": "ask1", "source": "agent", "text": f"Please merge armada/session-{id8} into main", "since": "t1"},
]}, "rows": []}
posts = []
class H(BaseHTTPRequestHandler):
    def log_message(self, *a): pass
    def do_GET(self):
        b = json.dumps(state).encode(); self.send_response(200); self.send_header("content-type", "application/json"); self.end_headers(); self.wfile.write(b)
    def do_POST(self):
        n = int(self.headers["content-length"]); body = json.loads(self.rfile.read(n)); posts.append((self.path, body))
        if self.path == "/sessions/waiting/answer":
            state["session"]["waiting_for"] = [i for i in state["session"]["waiting_for"] if i["id"] != body["item_id"]]
        self.send_response(200); self.send_header("content-type", "application/json"); self.end_headers(); self.wfile.write(b"{}")
srv = HTTPServer(("127.0.0.1", 0), H)
threading.Thread(target=srv.serve_forever, daemon=True).start()
json.dump({"port": srv.server_address[1], "pid": os.getpid()}, open(f"{home}/fleet.json", "w"))
open(f"{home}/armada.db", "w").close()  # person.py opens the store read only only for documents

def keeper(**kw):
    json.dump({"plan_version": 1, "gates_open": ["Custom state"], "gates_open_before_deliver": ["Custom state"], "merge_ready": False, "doc_open": False, "needs_you": [], "missing": {}, "held": None, **kw}, open(f"{run_dir}/state.json", "w"))

keeper()
p = subprocess.Popen([sys.executable, "-I", f"{here}/person.py", "watch", "--scratch", work, "--session", sid, "--acts", f"{run_dir}/acts.jsonl", "--run-dir", run_dir, "--rules", f"{here}/rules.json", "--max-minutes", "3", "--quiet-seconds", "8"], stdout=subprocess.PIPE, text=True)
def until(pred, what, secs=30):
    t = time.time()
    while time.time() - t < secs:
        if pred(): return
        time.sleep(0.3)
    raise SystemExit("timed out: " + what + "\n" + json.dumps(posts)[-600:])
refs = lambda: subprocess.run(["git", "--git-dir", f"{work}/origin.git", "rev-parse", "main"], capture_output=True, text=True).stdout.strip()
main0 = refs()
until(lambda: any(p_[0] == "/sessions/waiting/answer" for p_ in posts), "answer to the early merge ask")
ans = [p_ for p_ in posts if p_[0] == "/sessions/waiting/answer"][0][1]["text"]
assert "not merged" in ans and "Custom state" in ans, ans
assert refs() == main0, "main must not move while a gate before Deliver is open"
# (b) a held item on the document gate with nothing open
keeper(needs_you=[{"id": "hold-1", "gate": "document_approved", "title": "Write the design note", "text": "Held", "open": True}], missing={"document_approved": "no HTML page exists"})
until(lambda: any(p_[0] == "/sessions/message" and "Nothing for me to approve" in p_[1]["text"] for p_ in posts), "held answer")
assert not any(p_[0] == "/sessions/document/answer" for p_ in posts)
# (c) ready: a new ask is merged
keeper(merge_ready=True, gates_open=["Deliver"], gates_open_before_deliver=[])
state["session"]["waiting_for"].append({"id": "ask2", "source": "agent", "text": f"Please merge armada/session-{id8} into main", "since": "t2"})
until(lambda: refs() != main0, "the merge")
# (d) quiet with a done item present
state["session"]["waiting_for"] = [{"id": "done", "source": "done", "text": "done", "since": "x"}]
keeper(merge_ready=True, gates_open=["Deliver"], gates_open_before_deliver=[], held={"gate": "merged"})
out, _ = p.communicate(timeout=60)
assert out.strip() == "quiet", out
print("OK")
