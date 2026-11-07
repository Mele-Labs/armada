#!/usr/bin/env python3
"""The gate keeper for spike 49: Fleet's side of "a plan's groups, each ending on a gate". Stdlib only; run with `python3 -I`.

  gates.py --scratch DIR --session ID --run-dir DIR --template TREE --rig DIR [--poll 1]

Copy of 048-complex/gates.py with the six fixes from run 48:
  1. Every gate is watched on its own. Checks and the merge are looked at every poll, whatever the gates before them say.
     A merge is recorded even while an earlier gate is open (and logged as a mismatch).
  2. A gate names the exact form of its fact. document_approved closes only when the person approves a Fleet document: a ledger
     artifact of form `document` whose target is a page under `.armada/documents/*.html` and whose newest revision is settled
     `approved`. A file shown, Markdown, an answer in the thread never closes it (each is logged as ignored).
  3. Groups with no gate between them are one stretch. The keeper never closes a `none` group from task marks and writes no
     row for it. The live position is the first open gate, shown with its stretch; the agent's own marks inside the stretch
     are logged as its word only.
  4. A held gate is never silent. all_done, or a turn that ends with no item waiting, while a gate is open raises an item the
     person sees (state.json `needs_you`, logged as needs_you_raised: "would go in Needs you, with Approve and Skip this
     group") saying what is missing. A new all_done while held raises a new item.
  5. Edits (the agent's and its subagents', read from the transcripts) under a later group's paths while an earlier gate is
     open are logged as mismatch_edit. Subagents are logged: first prompt, edits, git acts, end.
  (6 is person.py.)

A group carries its gate as the last part of its `gives`: `[gate: plan_approved|document_approved|checks_green|merged|none]`.
Gates, by their fact:
  plan_approved      session_plan has approved_at at or after recorded_at for the current version
  document_approved  see 2
  checks_green       vitest on the test files the Job's branch adds or changes against the base main, at a pushed tip, green,
                     AND the tip holds a change under the paths of every group of the Checks stretch (so a green run at the
                     first of two pushes does not close it). Run on every new tip, whatever else is open
  merged             the branch tip (not the base) is an ancestor of origin's main
Each close is one row in gate_rows.jsonl, written once. Every event goes to keeper.jsonl. state.json is for the person's script.
Ends when <run-dir>/gates.stop exists, after one last pass.
"""
import argparse
import glob
import json
import os
import re
import sqlite3
import subprocess
import time
import urllib.request
from datetime import datetime, timezone

GATE_RX = re.compile(r"\[gate:\s*([A-Za-z_]+)\s*\]\s*$")
KNOWN = ("plan_approved", "document_approved", "checks_green", "merged", "none")
TEMPLATE_HEAD = (("Plan the change", "plan_approved"), ("Write the design note", "document_approved"))
TEMPLATE_TAIL = ("Deliver", "merged")
DOC_PATH = re.compile(r"/\.armada/documents/[^/]+\.html$")
GIT_ACT = re.compile(r"\bgit\s+(-C\s+\S+\s+)?(commit|push|merge|checkout|switch|branch|rebase)\b")


def now():
    return time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime()) + ".%03dZ" % (int(time.time() * 1000) % 1000)


def epoch(text):
    return datetime.fromisoformat(text.replace("Z", "+00:00")).astimezone(timezone.utc).timestamp()


def git(*args, cwd=None):
    r = subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True)
    return r.returncode, (r.stdout + r.stderr).strip() if r.returncode else r.stdout.strip()


def short(text, n=120):
    text = re.sub(r"\s+", " ", str(text)).strip()
    return text if len(text) <= n else text[: n - 1] + "…"


class Keeper:
    def __init__(self, a):
        self.a = a
        self.sid, self.id8 = a.session, a.session[:8]
        self.home = f"{a.scratch}/home/user"
        self.db = f"{self.home}/Library/Application Support/Armada/armada.db"
        runtime = json.load(open(f"{self.home}/Library/Application Support/Armada/fleet.json"))
        self.api = "http://127.0.0.1:%d" % runtime["port"]
        self.origin = f"{a.scratch}/origin.git"
        self.branch = f"armada/session-{self.id8}"
        self.check = f"{a.scratch}/verify-049-{self.id8}"
        self.run_dir = a.run_dir
        self.rows = []  # gate_rows, in order
        self.plan = None  # {"version", "recorded_at", "approved_at", "groups": [...], "tasks": {...}}
        self.seen_version = None
        self.refs = {}
        self.base_main = None  # origin's main when the keeper started
        self.own_pushes = []  # [{at, sha}]
        self.checked = {}  # sha -> (green, said, files, failed)
        self.tip_seen = (None, 0)  # a tip must sit still for one poll before Checks runs on it
        self.early_logged = set()
        self.doc_seen = {}
        self.ignored_logged = set()
        self.doc_closed_at = None
        self.live = "unset"
        self.agent_word = "unset"
        self.held = None  # {"gate", "title", "since", "reason", "item"}
        self.needs_you = []  # raised items; open until the gate closes or a newer item replaces it
        self.item_n = 0
        self.last_done = None
        self.last_running = None
        self.woke_watch = None
        self.state_cache = None
        self.offsets = {}  # transcript file -> bytes read
        self.subs = {}  # subagent transcript id -> {"first", "last", "prompt", "edits", "git", "calls"}
        self.edits_logged = set()
        self.mismatches = 0
        self.ledger_seen = set()
        self.uuids = set()
        self.stop_name = a.stop_name

    def resume(self):
        """Pick the record up again after a restart of the keeper in the same run: rows, base, pushes, transcript offsets."""
        rp = f"{self.run_dir}/gate_rows.jsonl"
        if os.path.exists(rp):
            self.rows = [json.loads(l) for l in open(rp)]
        for l in open(f"{self.run_dir}/keeper.jsonl"):
            e = json.loads(l)
            if e["event"] == "main_moved" and self.base_main is None:
                rc, full = git("--git-dir", self.origin, "rev-parse", e["sha"])
                self.base_main = full if rc == 0 else None
            if e["event"] == "push_own_branch":
                self.own_pushes.append({"at": e["at"], "sha": e["sha"]})
        doc = next((r for r in self.rows if r["gate"] == "document_approved"), None)
        self.doc_closed_at = doc["at"] if doc else None
        for path in glob.glob(f"{self.home}/.claude/projects/**/*.jsonl", recursive=True):
            if self.sid in path:
                self.offsets[path] = os.path.getsize(path)
        self.item_n = 100  # item ids of the restarted keeper do not repeat the first keeper's
        self.log_new_ledger_rows(quiet=True)
        plan = self.read_plan()
        if plan:
            self.seen_version = plan["version"]
        self.log("keeper_resumed", rows=len(self.rows), base_main=(self.base_main or "")[:10], pushes=len(self.own_pushes))

    # ---------------------------------------------------------------- output
    def log(self, event, **kw):
        with open(f"{self.run_dir}/keeper.jsonl", "a") as f:
            f.write(json.dumps({"at": now(), "event": event, **kw}) + "\n")

    def write_state(self, **kw):
        state = {"session": self.sid, "plan_version": self.plan["version"] if self.plan else None, **kw}
        if state != self.state_cache:
            self.state_cache = state
            tmp = f"{self.run_dir}/state.json.tmp"
            json.dump(state, open(tmp, "w"), indent=1)
            os.replace(tmp, f"{self.run_dir}/state.json")

    # ---------------------------------------------------------------- reads
    def fleet_one(self):
        try:
            with urllib.request.urlopen(f"{self.api}/sessions/one?session_id={self.sid}", timeout=20) as r:
                return json.load(r)["session"]
        except Exception as e:  # Fleet busy
            self.log("fleet_read_failed", error=str(e)[:100])
            return None

    def send(self, text):
        req = urllib.request.Request(self.api + "/sessions/message", json.dumps({"session_id": self.sid, "text": text}).encode(), {"content-type": "application/json"})
        try:
            with urllib.request.urlopen(req, timeout=30) as r:
                status = r.status
        except Exception as e:
            status = str(e)[:80]
        self.log("notice_sent", text=text, status=status)
        return status

    def store(self):
        con = sqlite3.connect(f"file:{self.db}?mode=ro", uri=True)
        con.row_factory = sqlite3.Row
        return con

    def read_plan(self):
        con = self.store()
        try:
            p = con.execute("select version, recorded_at, approved_at from session_plan where session_id=?", (self.sid,)).fetchone()
            if not p:
                return None
            groups = []
            for g in con.execute("select position, group_id, title, paths, gives from session_plan_groups where session_id=? order by position", (self.sid,)):
                gives = g["gives"] or ""
                m = GATE_RX.search(gives)
                try:
                    paths = json.loads(g["paths"] or "[]")
                except ValueError:
                    paths = [x for x in (g["paths"] or "").splitlines() if x]
                groups.append({"pos": g["position"], "id": g["group_id"], "title": g["title"], "gate": m.group(1) if m else None, "gives": gives, "paths": [str(x) for x in paths]})
            tasks = {}
            for t in con.execute("select group_id, task_id, listed, started_at, completed_at, marked from session_plan_tasks where session_id=? order by position", (self.sid,)):
                tasks.setdefault(t["group_id"], []).append({k: t[k] for k in t.keys()})
            return {"version": p["version"], "recorded_at": p["recorded_at"], "approved_at": p["approved_at"], "groups": groups, "tasks": tasks}
        finally:
            con.close()

    def read_artifacts(self):
        """The newest ledger version of each artifact attachment: {target: detail dict + _state/_at}."""
        con = self.store()
        out = {}
        try:
            for r in con.execute("select target, state, detail, at from ledger_attachment_versions where holder_id=? and kind='artifact' order by seq", (self.sid,)):
                try:
                    d = json.loads(r["detail"])
                except ValueError:
                    continue
                if isinstance(d, dict):
                    out[r["target"]] = {**d, "_state": r["state"], "_at": r["at"]}
        finally:
            con.close()
        return out

    def log_new_ledger_rows(self, quiet=False):
        """Subagent and other non-artifact rows Fleet's ledger keeps for this Job, logged once each."""
        con = self.store()
        try:
            for r in con.execute("select kind, target, state, detail, at from ledger_attachment_versions where holder_id=? and kind not in ('artifact','branch','slot') order by seq", (self.sid,)):
                key = (r["kind"], r["target"], r["state"])
                if key not in self.ledger_seen:
                    self.ledger_seen.add(key)
                    if not quiet:
                        self.log("ledger_row", kind=r["kind"], target=short(r["target"], 80), state=r["state"], detail=short(r["detail"], 160), ledger_at=r["at"])
        except sqlite3.Error:
            pass
        finally:
            con.close()

    def tip(self):
        rc, out = git("--git-dir", self.origin, "rev-parse", "--verify", "-q", f"refs/heads/{self.branch}")
        return out if rc == 0 and out else None

    # ---------------------------------------------------------------- the record
    def gated(self):
        """Groups that end on a real gate. A group with no gate (or `none`) belongs to the stretch that ends at the next one."""
        return [g for g in self.plan["groups"] if g["gate"] in KNOWN and g["gate"] != "none"]

    def stretch(self, g):
        gs = self.plan["groups"]
        i = j = gs.index(g)
        while j > 0 and (gs[j - 1]["gate"] not in KNOWN or gs[j - 1]["gate"] == "none"):
            j -= 1
        return gs[j : i + 1]

    def closed_row(self, g):
        if g["gate"] not in KNOWN or g["gate"] == "none":
            return None
        for r in self.rows:
            if r["gate"] != g["gate"]:
                continue
            if g["gate"] == "plan_approved":
                if r["plan_version"] == self.plan["version"]:
                    return r
            elif r["title"] == g["title"]:
                if g["gate"] == "checks_green" and r.get("stretch") != [x["title"] for x in self.stretch(g)]:
                    continue
                return r
        return None

    def close(self, g, fact, **extra):
        row = {"at": now(), "gate": g["gate"], "title": g["title"], "pos": g["pos"], "plan_version": self.plan["version"], "fact": fact, **extra}
        if g["gate"] == "checks_green":
            row["stretch"] = [x["title"] for x in self.stretch(g)]
        open_before = [x["title"] for x in self.gated() if x["pos"] < g["pos"] and self.closed_row(x) is None]
        self.rows.append(row)
        with open(f"{self.run_dir}/gate_rows.jsonl", "a") as f:
            f.write(json.dumps(row) + "\n")
        self.log("gate_closed", gate=g["gate"], title=g["title"], fact=fact, earlier_gates_open=open_before, **extra)
        if open_before:
            self.log("mismatch_gate_closed_out_of_order", gate=g["gate"], title=g["title"], earlier_open=open_before)

    def first_open(self):
        for g in self.gated():
            if self.closed_row(g) is None:
                return g
        return None

    # ---------------------------------------------------------------- the plan
    def template_check(self):
        gs = self.plan["groups"]
        problems = []
        for i, (title, gate) in enumerate(TEMPLATE_HEAD):
            if len(gs) <= i or gs[i]["title"] != title or gs[i]["gate"] != gate:
                problems.append(f"group {i + 1} should be '{title}' [{gate}]")
        if len(gs) < 5:
            problems.append(f"{len(gs)} groups: head two, at least one build group and Deliver need 4 or more")
        if gs and (gs[-1]["title"], gs[-1]["gate"]) != TEMPLATE_TAIL:
            problems.append("last group should be 'Deliver' [merged]")
        build = gs[2:-1]
        for i, g in enumerate(build):
            want = "checks_green" if i == len(build) - 1 else "none"
            if g["gate"] != want:
                problems.append(f"build group '{g['title']}' has [{g['gate']}], wanted [{want}]")
            if not g["paths"]:
                problems.append(f"build group '{g['title']}' has no paths")
        return problems

    def on_new_plan(self):
        gs = self.plan["groups"]
        self.log(
            "plan_recorded",
            version=self.plan["version"],
            groups=[{"pos": g["pos"], "title": g["title"], "gate": g["gate"], "paths": g["paths"], "tasks": len(self.plan["tasks"].get(g["id"], []))} for g in gs],
            template_problems=self.template_check(),
        )
        unknown = [g["title"] for g in gs if g["gate"] not in KNOWN]
        if unknown:
            self.log("gate_unknown_or_missing", groups=unknown, note="treated as none; a real Fleet would refuse the plan")
        if self.seen_version is not None:
            carried = [g["title"] for g in self.gated() if g["gate"] != "plan_approved" and self.closed_row(g)]
            reopened = [g["title"] for g in self.gated() if g["gate"] != "plan_approved" and not self.closed_row(g) and any(r["title"] == g["title"] for r in self.rows)]
            self.log("plan_gate_opened", version=self.plan["version"], carried_closed=carried, closed_before_but_open_now=reopened, open_now=[g["title"] for g in self.gated() if not self.closed_row(g)])
        self.seen_version = self.plan["version"]

    # ---------------------------------------------------------------- notices and holds
    def notice(self, g, text):
        status = self.send(text)
        if self.held and self.held["gate"] == g["gate"] and self.held.get("title") == g["title"]:
            self.log("held_released", gate=g["gate"], held_since=self.held["since"], seconds=round(time.time() - epoch(self.held["since"]), 1), reason=self.held["reason"], items=[i["id"] for i in self.needs_you if i["open"]])
            self.woke_watch = {"at": time.time(), "gate": g["gate"], "text": text}
            self.held = None
        elif self.woke_watch is None:
            self.woke_watch = {"at": time.time(), "gate": g["gate"], "text": text, "held": False}
        for i in self.needs_you:
            if i["open"] and i["gate"] == g["gate"] and i["title"] == g["title"]:
                i["open"] = False
                self.log("needs_you_cleared", id=i["id"], why="the gate closed")
        return status

    def missing(self, g):
        """What the gate is waiting for, in words the person reads."""
        if g["gate"] == "plan_approved":
            return f"the plan (version {self.plan['version']}) is waiting for your approval"
        if g["gate"] == "document_approved":
            arts = self.read_artifacts()
            docs = {t: d for t, d in arts.items() if d.get("form") == "document" and DOC_PATH.search(t)}
            if not docs:
                other = [os.path.basename(t) for t in arts]
                return "no HTML page exists under .armada/documents/, so there is nothing to approve" + (f" (shown so far: {', '.join(other)})" if other else "")
            t, d = sorted(docs.items(), key=lambda kv: kv[1]["_at"])[-1]
            return f"the page {os.path.basename(t)} revision {d.get('revision')} is not approved yet"
        if g["gate"] == "checks_green":
            last = list(self.checked.values())[-1] if self.checked else None
            return "Checks have not passed at a pushed commit that holds every build group: " + (last[1] if last else "nothing has been pushed to check")
        if g["gate"] == "merged":
            earlier = [x["title"] for x in self.gated() if x is not g and self.closed_row(x) is None]
            tip = self.tip()
            return f"the branch tip {tip[:7] if tip else '(none)'} is not on main" + (f"; still open before it: {', '.join(earlier)}" if earlier else "")
        return "the gate's fact has not happened"

    def hold(self, g, reason, fresh_done=False):
        """Hold the Job on this gate and raise a Needs you item for the person: never silent."""
        if self.held and not fresh_done:
            open_item = next((i for i in self.needs_you if i["open"]), None)
            if open_item is not None and open_item["text"] == f"Held on {g['title']}: {self.missing(g)}":
                self.log("hold_again", gate=g["gate"], reason=reason, already=self.held["gate"])
                return
            self.log("hold_text_changed", gate=g["gate"], reason=reason, was=open_item["text"] if open_item else None)
        if self.held is None:
            self.held = {"gate": g["gate"], "title": g["title"], "since": now(), "reason": reason}
        self.item_n += 1
        for i in self.needs_you:
            if i["open"]:
                i["open"] = False
                self.log("needs_you_cleared", id=i["id"], why="a newer item replaces it")
        item = {
            "id": f"hold-{self.item_n}",
            "gate": g["gate"],
            "title": g["title"],
            "text": f"Held on {g['title']}: {self.missing(g)}",
            "options": ["Approve", "Skip this group"],
            "raised_at": now(),
            "reason": reason,
            "open": True,
        }
        self.needs_you.append(item)
        self.held["item"] = item["id"]
        self.log("held", gate=g["gate"], title=g["title"], reason=reason, item=item["id"])
        self.log("needs_you_raised", **{k: item[k] for k in ("id", "gate", "title", "text", "options", "reason")}, note="would go in Needs you, with Approve and Skip this group")

    # ---------------------------------------------------------------- Checks
    def changed_files(self, sha):
        rc, names = git("--git-dir", self.origin, "diff", "--name-only", self.base_main, sha)
        return names.splitlines() if rc == 0 else []

    def run_checks(self, sha):
        if not os.path.isdir(self.check):
            git("clone", "--quiet", "--local", self.origin, self.check)
            subprocess.run(["bash", f"{self.a.rig}/seed-node-modules.sh", self.a.template], cwd=self.check, capture_output=True)
        git("fetch", "--quiet", "origin", cwd=self.check)
        git("checkout", "--quiet", "--detach", "-f", sha, cwd=self.check)
        git("clean", "-fdq", cwd=self.check)
        files = [n for n in self.changed_files(sha) if re.search(r"\.test\.(ts|tsx)$", n) and os.path.exists(f"{self.check}/{n}")]
        if not files:
            return False, "no test file added or changed on the branch", [], ["no test file"]
        groups = {}
        for f in files:
            parts = f.split("/")
            root = "/".join(parts[:2]) if parts[0] in ("apps", "packages") else ""
            if not os.path.exists(f"{self.check}/{root}/node_modules/.bin/vitest"):
                root = "apps/desktop"
            groups.setdefault(root, []).append(f[len(root) + 1 :] if root else f)
        t0 = time.time()
        failed, lines = [], []
        for root, rel in groups.items():
            r = subprocess.run([f"{self.check}/{root}/node_modules/.bin/vitest", "run", *rel], cwd=f"{self.check}/{root}", capture_output=True, text=True, timeout=600)
            text = r.stdout + r.stderr
            line = next((l.strip() for l in text.splitlines() if l.strip().startswith("Tests ")), "no Tests line")
            lines.append(line)
            if not (r.returncode == 0 and " passed" in line and " failed" not in line):
                failed += [l.split()[1] + (" " + l.split(">", 1)[1].strip() if ">" in l else "") for l in text.splitlines() if l.strip().startswith("FAIL ") and len(l.split()) > 1][:4] or rel[:3]
        said = f"vitest on {len(files)} file(s) ({', '.join(os.path.basename(f) for f in files)}): {'; '.join(lines)} ({time.time() - t0:.0f}s)"
        return not failed, said, files, failed

    @staticmethod
    def covers(path, f):
        p = path.strip().rstrip("/")
        return bool(p) and (f == p or f.endswith("/" + p) or f.startswith(p + "/") or ("/" + p + "/") in f)

    def stretch_missing(self, g, sha):
        """The groups of g's stretch that have no change under their paths at this tip."""
        files = self.changed_files(sha)
        return [grp["title"] for grp in self.stretch(g) if grp["paths"] and not any(self.covers(p, f) for p in grp["paths"] for f in files)]

    def checks_poll(self):
        tip = self.tip()
        checks_g = next((g for g in self.plan["groups"] if g["gate"] == "checks_green"), None)
        if not tip or tip == self.base_main:
            return
        if self.tip_seen[0] != tip:
            self.tip_seen = (tip, time.time())
            return  # let a push sit one poll before running anything
        if tip not in self.checked:
            self.checked[tip] = self.run_checks(tip)
            green, said, files, failed = self.checked[tip]
            self.log("checks_run", sha=tip[:10], green=green, said=said, failed=failed, live_group=(self.live[1] if isinstance(self.live, tuple) else None))
        green, said, files, failed = self.checked[tip]
        if checks_g is None or self.closed_row(checks_g):
            return
        lacking = self.stretch_missing(checks_g, tip)
        if green and not lacking:
            self.close(checks_g, f"{said} at {tip[:10]}", sha=tip)
            self.notice(checks_g, f"Fleet: Checks passed at {tip[:7]}.")
        elif green:
            if (tip, tuple(lacking), checks_g["title"]) not in self.early_logged:
                self.early_logged.add((tip, tuple(lacking), checks_g["title"]))
                self.log("checks_green_but_stretch_incomplete", sha=tip[:10], lacking=lacking, note="not a close: no change under these groups' paths at this tip")
        elif not lacking and (tip, "red") not in self.early_logged:
            self.early_logged.add((tip, "red"))
            if self.held and self.held["gate"] == "checks_green":
                self.send(f"Fleet: Checks failed at {tip[:7]}: {', '.join(failed)}.")

    # ---------------------------------------------------------------- transcripts: edits and subagents
    def transcripts_poll(self):
        files = [f for f in glob.glob(f"{self.home}/.claude/projects/**/*.jsonl", recursive=True) if self.sid in f]
        for path in sorted(files):
            sub = os.path.basename(path)[:-6] if "/subagents/" in path else None
            try:
                size = os.path.getsize(path)
            except OSError:
                continue
            off = self.offsets.get(path, 0)
            if size <= off:
                continue
            with open(path, "rb") as f:
                f.seek(off)
                chunk = f.read()
            nl = chunk.rfind(b"\n")
            if nl < 0:
                continue
            self.offsets[path] = off + nl + 1
            for line in chunk[: nl + 1].splitlines():
                try:
                    d = json.loads(line)
                except ValueError:
                    continue
                self.on_transcript_line(d, sub)

    def on_transcript_line(self, d, sub):
        at = d.get("timestamp")
        msg = d.get("message")
        if not at or not isinstance(msg, dict):
            return
        if d.get("uuid"):
            if d["uuid"] in self.uuids:
                return
            self.uuids.add(d["uuid"])
        rec = None
        if sub:
            rec = self.subs.setdefault(sub, {"first": at, "last": at, "prompt": None, "edits": [], "git": [], "calls": 0})
            rec["last"] = at
            if rec["prompt"] is None and d.get("type") == "user":
                c = msg.get("content")
                rec["prompt"] = short(c if isinstance(c, str) else " ".join(x.get("text", "") for x in c if isinstance(x, dict)), 240)
                self.log("subagent_seen", agent=sub, started=at, prompt=rec["prompt"])
        content = msg.get("content")
        if d.get("type") != "assistant" or not isinstance(content, list):
            return
        who = f"sub:{sub}" if sub else "agent"
        for b in content:
            if b.get("type") != "tool_use":
                continue
            name, inp = b.get("name"), b.get("input") or {}
            if rec is not None:
                rec["calls"] += 1
            if name in ("Agent", "Task"):
                self.log("agent_tool_call", by=who, subagent_type=inp.get("subagent_type"), description=short(inp.get("description", ""), 80), background=inp.get("run_in_background"), transcript_at=at)
            elif name in ("Edit", "Write", "MultiEdit", "NotebookEdit"):
                self.on_edit(who, inp.get("file_path") or inp.get("notebook_path") or "", at, rec)
            elif name == "Bash":
                cmd = inp.get("command", "")
                if GIT_ACT.search(cmd):
                    if rec is not None:
                        rec["git"].append(short(cmd, 100))
                    self.log("git_act", by=who, command=short(cmd, 160), transcript_at=at, live_group=(self.live[1] if isinstance(self.live, tuple) else None))

    def group_of(self, path):
        """The group whose paths cover this file, else None. A page under .armada/documents belongs to the document group."""
        if DOC_PATH.search(path):
            return next((g for g in self.plan["groups"] if g["gate"] == "document_approved"), None)
        for g in self.plan["groups"]:
            if any(self.covers(p, path) for p in g["paths"]):
                return g
        return None

    def on_edit(self, who, path, at, rec):
        if not path or self.plan is None:
            return
        if rec is not None:
            rec["edits"].append(path)
        if (who, path) in self.edits_logged:
            return
        self.edits_logged.add((who, path))
        g = self.group_of(path)
        live = self.first_open()
        rel = path.split("/slots/", 1)[-1] if "/slots/" in path else path
        if g is None:
            self.log("edit_outside_every_group", by=who, path=rel, live=(live or {}).get("title"), transcript_at=at)
        elif live is not None and g["pos"] > live["pos"]:
            self.mismatches += 1
            self.log("mismatch_edit", by=who, path=rel, edit_group=g["title"], live_gate=live["title"], gate=live["gate"], transcript_at=at)
        else:
            self.log("edit_seen", by=who, path=rel, group=g["title"], live_gate=(live or {}).get("title"), transcript_at=at)

    # ---------------------------------------------------------------- the loop
    def poll(self):
        plan = self.read_plan()
        if plan is not None:
            if self.plan is None or plan["version"] != self.plan["version"]:
                self.plan = plan
                self.on_new_plan()
            else:
                self.plan = plan
        self.refs_poll()
        self.log_new_ledger_rows()
        if self.plan is None:
            return
        sess = self.fleet_one()
        done = (sess or {}).get("done_since")
        running = ((sess or {}).get("hosted") or {}).get("turn", {}).get("state") not in (None, "idle")
        items = [i for i in ((sess or {}).get("waiting_for") or []) if i.get("source") != "done"]
        self.transcripts_poll()
        gs = self.plan["groups"]

        # --- plan gate (a new version is a new fact) ---
        for g in gs:
            if g["gate"] == "plan_approved" and self.closed_row(g) is None:
                ap, rec = self.plan["approved_at"], self.plan["recorded_at"]
                if ap and epoch(ap) >= epoch(rec) - 0.001:
                    self.close(g, f"session_plan v{self.plan['version']} approved_at {ap}")
                    self.notice(g, "Fleet: the person approved the plan." if self.plan["version"] == 1 else f"Fleet: the person approved plan version {self.plan['version']}.")
                elif ap:
                    self.log("plan_approved_at_before_recorded_at", approved_at=ap, recorded_at=rec, version=self.plan["version"])

        # --- document gate: only a Fleet document under .armada/documents/*.html, approved on its newest revision ---
        arts = self.read_artifacts()
        for target, d in arts.items():
            if d.get("form") != "document":
                if ("notdoc", target) not in self.ignored_logged:
                    self.ignored_logged.add(("notdoc", target))
                    self.log("artifact_ignored", target=target, form=d.get("form"), why="not a document: cannot close document_approved")
                continue
            if not DOC_PATH.search(target):
                if ("path", target) not in self.ignored_logged:
                    self.ignored_logged.add(("path", target))
                    self.log("artifact_ignored", target=target, form="document", why="not under .armada/documents/*.html")
                continue
            try:
                revisions = json.loads(d.get("revisions", "[]"))
            except ValueError:
                revisions = []
            key = (d.get("revision"), d.get("settled"), d.get("asked"), len(revisions))
            if self.doc_seen.get(target) != key:
                self.doc_seen[target] = key
                self.log("document", target=target, revision=d.get("revision"), revisions=len(revisions), asked=d.get("asked"), settled=d.get("settled"), words=d.get("words"))
            if d.get("settled") == "approved" and d.get("asked") == "true" and d.get("revision") == str(len(revisions)):
                for g in gs:
                    if g["gate"] == "document_approved" and self.closed_row(g) is None:
                        self.close(g, f"document {target} revision {d.get('revision')} of {len(revisions)} approved", revision=d.get("revision"), target=target)
                        self.doc_closed_at = now()
                        self.notice(g, "Fleet: the person approved the design note.")

        # --- Checks and the merge, each on its own ---
        self.checks_poll()
        merged_g = next((g for g in gs if g["gate"] == "merged"), None)
        tip = self.tip()
        if merged_g and self.closed_row(merged_g) is None and tip and tip != self.base_main:
            rc, _ = git("--git-dir", self.origin, "merge-base", "--is-ancestor", tip, "refs/heads/main")
            if rc == 0:
                self.close(merged_g, f"{tip[:10]} is an ancestor of origin/main", sha=tip)
                checks_row = next((r for r in self.rows if r["gate"] == "checks_green"), None)
                if checks_row and checks_row["sha"] != tip:
                    self.log("branch_moved_after_checks", checked=checks_row["sha"][:10], tip=tip[:10])
                self.notice(merged_g, "Fleet: the branch is merged into main.")

        # --- the agent called all_done ---
        if done and done != self.last_done:
            self.last_done = done
            open_g = self.first_open()
            self.log("all_done_seen", since=done, first_open=(open_g or {}).get("title"), gate=(open_g or {}).get("gate"))
            if open_g is not None:
                self.hold(open_g, "all_done", fresh_done=True)
        elif done is None:
            self.last_done = None

        # --- a turn that ended with a gate open and nothing asked of the person ---
        if self.last_running is True and not running:
            open_g = self.first_open()
            self.log("turn_ended", first_open=(open_g or {}).get("title"), gate=(open_g or {}).get("gate"), waiting=[i["id"] for i in items][:4], held=bool(self.held))
            if open_g is not None and not items and not done:
                self.hold(open_g, "turn_end")
        if running and self.woke_watch is not None and self.last_running is False:
            self.log("woke", after_seconds=round(time.time() - self.woke_watch["at"], 1), by=self.woke_watch["gate"], held=self.woke_watch.get("held", True))
            self.woke_watch = None
        self.last_running = running
        if self.woke_watch and time.time() - self.woke_watch["at"] > 90 and not running:
            self.log("no_wake_90s", by=self.woke_watch["gate"])
            self.woke_watch = None

        # --- the live position: the first open gate and its stretch; the agent's marks are its word only ---
        open_g = self.first_open()
        live = (open_g["pos"], open_g["title"], open_g["gate"]) if open_g else None
        if live != self.live:
            self.live = live
            self.log("live_group", pos=live[0] if live else None, title=live[1] if live else "(all gates closed)", gate=live[2] if live else None, stretch=[x["title"] for x in self.stretch(open_g)] if open_g else [], plan_version=self.plan["version"])
        word = self.agent_word_group()
        if word != self.agent_word:
            self.agent_word = word
            self.log("agent_word_group", group=word, note="the agent's own task marks; not a position")

        gated_open = [g["title"] for g in self.gated() if not self.closed_row(g)]
        before_deliver_open = [g["title"] for g in self.gated() if g["gate"] != "merged" and not self.closed_row(g)]
        checks_g = next((g for g in gs if g["gate"] == "checks_green"), None)
        checks_row = self.closed_row(checks_g) if checks_g else None
        merge_ready = bool(not before_deliver_open and tip and tip != self.base_main and checks_row and checks_row.get("sha") == tip)
        docs_open = [t for t, d in arts.items() if d.get("form") == "document" and DOC_PATH.search(t) and d.get("settled") != "approved"]
        after_doc = [p for p in self.own_pushes if self.doc_closed_at and p["at"] > self.doc_closed_at]
        self.write_state(
            doc_closed_at=self.doc_closed_at,
            own_pushes=len(self.own_pushes),
            own_pushes_after_doc=len(after_doc),
            live=list(live) if live else None,
            gates_open=gated_open,
            gates_open_before_deliver=before_deliver_open,
            merge_ready=merge_ready,
            merged=bool(merged_g and self.closed_row(merged_g)),
            doc_open=bool(docs_open),
            tip=tip[:10] if tip else None,
            missing={g["gate"]: self.missing(g) for g in self.gated() if not self.closed_row(g)},
            held=self.held,
            needs_you=[i for i in self.needs_you if i["open"]],
        )

    def agent_word_group(self):
        best = None
        for g in self.plan["groups"]:
            for t in self.plan["tasks"].get(g["id"], []):
                if t["started_at"] and not t["completed_at"] and t["listed"] != "completed":
                    best = g["title"]
        return best

    def refs_poll(self):
        rc, out = git("--git-dir", self.origin, "for-each-ref", "--format=%(refname:short) %(objectname)", "refs/heads")
        if rc != 0:
            return
        cur = dict(l.split(" ", 1) for l in out.splitlines() if " " in l)
        if self.base_main is None and "main" in cur:
            self.base_main = cur["main"]
        for ref, sha in cur.items():
            if self.refs.get(ref) != sha:
                own = ref == self.branch
                kind = "main_moved" if ref == "main" else ("push_own_branch" if own else "push_other_branch")
                n = git("--git-dir", self.origin, "rev-list", "--count", f"{self.base_main}..{sha}")[1] if ref != "main" else ""
                live = self.live if isinstance(self.live, tuple) else None
                self.log(kind, ref=ref, sha=sha[:10], previous=(self.refs.get(ref) or "")[:10], commits_past_main=n, live_group=live[1] if live else None)
                if own:
                    self.own_pushes.append({"at": now(), "sha": sha})
        self.refs = cur

    def final_summary(self):
        for sub, r in self.subs.items():
            self.log(
                "subagent_summary",
                agent=sub,
                started=r["first"],
                ended=r["last"],
                seconds=round(epoch(r["last"]) - epoch(r["first"]), 1),
                tool_calls=r["calls"],
                edits=sorted(set(r["edits"])),
                git=r["git"],
                prompt=r["prompt"],
            )
        self.log("mismatch_total", mismatch_edits=self.mismatches)

    def run(self):
        self.log("keeper_started", branch=self.branch, template=[t for t in TEMPLATE_HEAD] + [TEMPLATE_TAIL])
        while True:
            stopping = os.path.exists(f"{self.run_dir}/{self.stop_name}")
            try:
                self.poll()
            except Exception as e:  # the keeper must not die mid-run
                import traceback

                self.log("keeper_error", error=repr(e)[:300], where=traceback.format_exc()[-300:])
            if stopping:
                self.final_summary()
                self.log("keeper_stopped", open=[g["title"] for g in (self.gated() if self.plan else []) if self.closed_row(g) is None], held=self.held)
                return
            time.sleep(self.a.poll)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--scratch", required=True)
    ap.add_argument("--session", required=True)
    ap.add_argument("--run-dir", required=True)
    ap.add_argument("--template", required=True, help="a tree with node_modules to copy from")
    ap.add_argument("--rig", required=True, help="the 042-rig directory (seed-node-modules.sh)")
    ap.add_argument("--poll", type=float, default=1)
    ap.add_argument("--resume", action="store_true", help="continue a run whose keeper was stopped")
    ap.add_argument("--stop-name", default="gates.stop")
    k = Keeper(ap.parse_args())
    if k.a.resume:
        k.resume()
    k.run()


if __name__ == "__main__":
    main()
