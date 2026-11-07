#!/usr/bin/env python3
"""The gate keeper for spike 48: Fleet's side of "a plan's groups, each ending on a gate". Stdlib only; run with `python3 -I`.

  gates.py --scratch DIR --session ID --run-dir DIR --template TREE --rig DIR [--poll 1]

Copy of 046-refusals/gates.py, rebuilt. Differences:
  - the gates belong to the groups of the plan the agent recorded. A group carries its gate as the last part of its
    `gives`, as `[gate: plan_approved|document_approved|checks_green|merged|none]` (record_plan drops an unknown field,
    so the text carries it). The keeper reads session_plan_groups on every poll and never takes the agent's word for
    a gate being closed.
  - each close is written once to <run-dir>/gate_rows.jsonl with the fact that closed it, and never re-derived.
    A newly recorded plan version opens a new plan gate; every other gate stays closed when its group's title carries over.
  - when a gate closes, the agent gets ONE line through POST /sessions/message, prefixed "Fleet:", stating the fact.
    A `none` gate closes silently. A failed Checks run is told only while the Job is held on that gate.
  - all_done (or a turn that ends with nothing asked of the person) while a gate is open is recorded as "held on <gate>";
    the notice sent when that gate closes is what wakes the Job. No refusal of anything.
  - Checks: vitest on the test files the Job's own branch adds or changes against main, at the branch tip on the bare
    origin, in a scratch checkout. Run only once every group before the Checks group has closed.
  - every ref that moves on the bare origin is logged, the Job's own branch (armada/session-<id8>) apart from others.

Gates, by their fact:
  plan_approved      session_plan has approved_at at or after recorded_at for the current version
  document_approved  a document attachment (ledger kind artifact, detail.form=document) whose newest revision is settled approved
  checks_green       the Checks run above exits 0 with a passing count and no failure
  merged             the commit that passed Checks is an ancestor of origin's main
  none               the next group has started (a task of it is marked), or a later gate has closed, or the agent called all_done

Writes in <run-dir>: gate_rows.jsonl (the record), keeper.jsonl (every event), state.json (for the person's script).
Ends when <run-dir>/gates.stop exists, after one last pass.
"""
import argparse
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


def now():
    return time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime()) + ".%03dZ" % (int(time.time() * 1000) % 1000)


def epoch(text):
    return datetime.fromisoformat(text.replace("Z", "+00:00")).astimezone(timezone.utc).timestamp()


def git(*args, cwd=None):
    r = subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True)
    return r.returncode, (r.stdout + r.stderr).strip() if r.returncode else r.stdout.strip()


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
        self.check = f"{a.scratch}/verify-048-{self.id8}"
        self.run_dir = a.run_dir
        # the record
        self.rows = []  # gate_rows, in order
        self.plan = None  # {"version", "recorded_at", "approved_at", "groups": [...]}
        self.seen_version = None
        self.refs = {}
        self.own_pushes = []  # [{at, sha}]
        self.checked_tips = {}  # sha -> (green, said)
        self.last_tip_checked = None
        self.doc_seen = {}  # target -> (revision, settled, asked)
        self.doc_closed_at = None
        self.live = "unset"
        self.held = None  # {"gate", "since", "reason"}
        self.last_done = None
        self.last_running = None
        self.running_since = None
        self.woke_watch = None  # {"at": epoch of notice, "gate"}
        self.state_cache = None
        self.cleaned_up = False

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
            for g in con.execute("select position, group_id, title, gives from session_plan_groups where session_id=? order by position", (self.sid,)):
                gives = g["gives"] or ""
                m = GATE_RX.search(gives)
                gate = m.group(1) if m else None
                groups.append({"pos": g["position"], "id": g["group_id"], "title": g["title"], "gate": gate, "gives": gives})
            tasks = {}
            for t in con.execute("select group_id, task_id, listed, started_at, completed_at, marked from session_plan_tasks where session_id=? order by position", (self.sid,)):
                tasks.setdefault(t["group_id"], []).append({k: t[k] for k in t.keys()})
            return {"version": p["version"], "recorded_at": p["recorded_at"], "approved_at": p["approved_at"], "groups": groups, "tasks": tasks}
        finally:
            con.close()

    def read_documents(self):
        """The newest ledger version of each document attachment: {target: detail dict}."""
        con = self.store()
        out = {}
        try:
            for r in con.execute("select target, state, detail, at from ledger_attachment_versions where holder_id=? and kind='artifact' order by seq", (self.sid,)):
                try:
                    d = json.loads(r["detail"])
                except ValueError:
                    continue
                if isinstance(d, dict) and d.get("form") == "document":
                    out[r["target"]] = {**d, "_state": r["state"], "_at": r["at"]}
        finally:
            con.close()
        return out

    def tip(self):
        rc, out = git("--git-dir", self.origin, "rev-parse", "--verify", "-q", f"refs/heads/{self.branch}")
        return out if rc == 0 and out else None

    # ---------------------------------------------------------------- the record
    def closed_row(self, g):
        for r in self.rows:
            if g["gate"] == "plan_approved":
                if r["gate"] == "plan_approved" and r["plan_version"] == self.plan["version"]:
                    return r
            elif r["gate"] == g["gate"] and r["title"] == g["title"]:
                return r
        return None

    def close(self, g, fact, **extra):
        row = {"at": now(), "gate": g["gate"], "title": g["title"], "pos": g["pos"], "plan_version": self.plan["version"], "fact": fact, **extra}
        self.rows.append(row)
        with open(f"{self.run_dir}/gate_rows.jsonl", "a") as f:
            f.write(json.dumps(row) + "\n")
        self.log("gate_closed", gate=g["gate"], title=g["title"], fact=fact, **extra)

    def first_open(self):
        for g in self.plan["groups"]:
            if self.closed_row(g) is None:
                return g
        return None

    def started(self, g):
        for t in self.plan["tasks"].get(g["id"], []):
            if t["started_at"] or t["completed_at"] or t["marked"] or t["listed"] in ("in_progress", "completed"):
                return True
        return False

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
        return problems

    def on_new_plan(self):
        gs = self.plan["groups"]
        self.log(
            "plan_recorded",
            version=self.plan["version"],
            groups=[{"pos": g["pos"], "title": g["title"], "gate": g["gate"], "tasks": len(self.plan["tasks"].get(g["id"], []))} for g in gs],
            template_problems=self.template_check(),
        )
        unknown = [g["title"] for g in gs if g["gate"] not in KNOWN]
        if unknown:
            self.log("gate_unknown_or_missing", groups=unknown, note="treated as none; a real Fleet would refuse the plan")
            for g in gs:
                if g["gate"] not in KNOWN:
                    g["gate"] = "none"
        if self.seen_version is not None:
            carried = [g["title"] for g in gs if g["gate"] != "plan_approved" and self.closed_row(g)]
            reopened = [g["title"] for g in gs if g["gate"] != "plan_approved" and not self.closed_row(g) and any(r["title"] == g["title"] for r in self.rows)]
            self.log("plan_gate_opened", version=self.plan["version"], carried_closed=carried, closed_before_but_open_now=reopened)
        self.seen_version = self.plan["version"]

    # ---------------------------------------------------------------- notices
    def notice(self, g, text):
        status = self.send(text)
        if self.held and self.held["gate"] == g["gate"] and self.held.get("title") == g["title"]:
            self.log("held_released", gate=g["gate"], held_since=self.held["since"], seconds=round(time.time() - epoch(self.held["since"]), 1), reason=self.held["reason"])
            self.woke_watch = {"at": time.time(), "gate": g["gate"], "text": text}
            self.held = None
        elif self.woke_watch is None:
            self.woke_watch = {"at": time.time(), "gate": g["gate"], "text": text, "held": False}
        return status

    # ---------------------------------------------------------------- checks
    def run_checks(self, sha):
        if not os.path.isdir(self.check):
            git("clone", "--quiet", "--local", self.origin, self.check)
            subprocess.run(["bash", f"{self.a.rig}/seed-node-modules.sh", self.a.template], cwd=self.check, capture_output=True)
        git("fetch", "--quiet", "origin", cwd=self.check)
        git("checkout", "--quiet", "--detach", "-f", sha, cwd=self.check)
        git("clean", "-fdq", cwd=self.check)
        rc, names = git("diff", "--name-only", f"origin/main...{sha}", cwd=self.check)
        files = [n for n in names.splitlines() if re.search(r"\.test\.(ts|tsx)$", n) and os.path.exists(f"{self.check}/{n}")]
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
                failed += [l.strip().split(" ")[1] for l in text.splitlines() if l.strip().startswith("FAIL ")][:4] or rel[:3]
        said = f"vitest on {len(files)} file(s) ({', '.join(os.path.basename(f) for f in files)}): {'; '.join(lines)} ({time.time() - t0:.0f}s)"
        return not failed, said, files, failed

    # ---------------------------------------------------------------- the loop
    def poll(self, stopping):
        plan = self.read_plan()
        if plan is not None:
            if self.plan is None or plan["version"] != self.plan["version"]:
                self.plan = plan
                self.on_new_plan()
            else:
                self.plan = plan
                for g in plan["groups"]:
                    if g["gate"] not in KNOWN:
                        g["gate"] = "none"
        if self.plan is None:
            self.refs_poll()
            return
        sess = self.fleet_one()
        done = (sess or {}).get("done_since")
        running = ((sess or {}).get("hosted") or {}).get("turn", {}).get("state") not in (None, "idle")
        items = [i for i in ((sess or {}).get("waiting_for") or []) if i.get("source") != "done"]
        self.refs_poll()

        # --- the plan gate (a new version is a new fact) ---
        for g in self.plan["groups"]:
            if g["gate"] == "plan_approved" and self.closed_row(g) is None:
                ap, rec = self.plan["approved_at"], self.plan["recorded_at"]
                if ap and epoch(ap) >= epoch(rec) - 0.001:
                    self.close(g, f"session_plan v{self.plan['version']} approved_at {ap}")
                    self.notice(g, "Fleet: the person approved the plan." if self.plan["version"] == 1 else f"Fleet: the person approved plan version {self.plan['version']}.")
                elif ap:
                    self.log("plan_approved_at_before_recorded_at", approved_at=ap, recorded_at=rec, version=self.plan["version"])

        # --- documents: log each change, close the document gate on an approved newest revision ---
        docs = self.read_documents()
        for target, d in docs.items():
            try:
                revisions = json.loads(d.get("revisions", "[]"))
            except ValueError:
                revisions = []
            key = (d.get("revision"), d.get("settled"), d.get("asked"), len(revisions))
            if self.doc_seen.get(target) != key:
                self.doc_seen[target] = key
                self.log("document", target=target, revision=d.get("revision"), revisions=len(revisions), asked=d.get("asked"), settled=d.get("settled"), words=d.get("words"))
            if d.get("settled") == "approved" and d.get("asked") == "true" and d.get("revision") == str(len(revisions)):
                for g in self.plan["groups"]:
                    if g["gate"] == "document_approved" and self.closed_row(g) is None:
                        self.close(g, f"document {target} revision {d.get('revision')} of {len(revisions)} approved", revision=d.get("revision"))
                        self.doc_closed_at = now()
                        self.notice(g, "Fleet: the person approved the design note.")

        # --- none gates: the next group has started, or a later gate has closed ---
        gs = self.plan["groups"]
        for i, g in enumerate(gs):
            if g["gate"] != "none" or self.closed_row(g):
                continue
            if any(self.closed_row(later) for later in gs[i + 1 :]):
                self.close(g, "a later group's gate has closed")
            elif i + 1 < len(gs) and self.started(gs[i + 1]):
                self.close(g, f"the next group '{gs[i + 1]['title']}' has a task marked started or done (the agent's word)")
            else:
                break  # groups close in order

        # --- the agent called all_done ---
        if done and done != self.last_done:
            self.last_done = done
            open_g = self.first_open()
            self.log("all_done_seen", since=done, first_open=(open_g or {}).get("title"), gate=(open_g or {}).get("gate"))
            while open_g is not None and open_g["gate"] == "none":
                self.close(open_g, "the agent called all_done with this group's gate open")
                open_g = self.first_open()
            if open_g is not None:
                self.hold(open_g, "all_done")
        elif done is None:
            self.last_done = None

        # --- a turn that ended with a gate open and nothing asked of the person ---
        if self.last_running is True and not running:
            open_g = self.first_open()
            self.log("turn_ended", first_open=(open_g or {}).get("title"), gate=(open_g or {}).get("gate"), waiting=[i["id"] for i in items][:4], held=bool(self.held))
            if open_g is not None and not items and open_g["gate"] in ("checks_green", "merged", "document_approved") and not done:
                self.hold(open_g, "turn_end")
        if self.last_running is False and running and self.running_since is None:
            self.running_since = time.time()
        if running:
            if self.woke_watch is not None and self.last_running is False:
                self.log("woke", after_seconds=round(time.time() - self.woke_watch["at"], 1), by=self.woke_watch["gate"], held=self.woke_watch.get("held", True))
                self.woke_watch = None
        self.last_running = running
        if self.woke_watch and time.time() - self.woke_watch["at"] > 90 and not running:
            self.log("no_wake_90s", by=self.woke_watch["gate"])
            self.woke_watch = None

        # --- Checks, once every group before the Checks group has closed ---
        checks_g = next((g for g in gs if g["gate"] == "checks_green"), None)
        if checks_g and self.closed_row(checks_g) is None:
            before = gs[: gs.index(checks_g)]
            tip = self.tip()
            if tip and all(self.closed_row(b) for b in before):
                if tip != self.last_tip_checked:
                    self.last_tip_checked = tip
                    green, said, files, failed = self.run_checks(tip)
                    self.log("checks_run", sha=tip[:10], green=green, said=said, failed=failed)
                    if green:
                        self.close(checks_g, f"{said} at {tip[:10]}", sha=tip)
                        self.notice(checks_g, f"Fleet: Checks passed at {tip[:7]}.")
                    elif self.held and self.held["gate"] == "checks_green":
                        self.send(f"Fleet: Checks failed at {tip[:7]}: {', '.join(failed)}.")
            elif tip and tip != self.last_tip_checked and not stopping:
                if getattr(self, "_not_due", None) != tip:
                    self._not_due = tip
                    self.log("checks_not_due", sha=tip[:10], first_open=(self.first_open() or {}).get("title"))

        # --- merged ---
        merged_g = next((g for g in gs if g["gate"] == "merged"), None)
        if merged_g and self.closed_row(merged_g) is None and checks_g and self.closed_row(checks_g):
            sha = next(r["sha"] for r in self.rows if r["gate"] == "checks_green")
            rc, _ = git("--git-dir", self.origin, "merge-base", "--is-ancestor", sha, "refs/heads/main")
            if rc == 0:
                self.close(merged_g, f"{sha[:10]} is an ancestor of origin/main")
                tip = self.tip()
                if tip and tip != sha:
                    self.log("branch_moved_after_checks", checked=sha[:10], tip=tip[:10])
                self.notice(merged_g, "Fleet: the branch is merged into main.")

        # --- the live group: the first group whose gate is open ---
        open_g = self.first_open()
        live = (open_g["pos"], open_g["title"], open_g["gate"]) if open_g else None
        if live != self.live:
            self.live = live
            self.log("live_group", pos=live[0] if live else None, title=live[1] if live else "(all gates closed)", gate=live[2] if live else None, plan_version=self.plan["version"])
        first_build = gs[2] if len(gs) > 3 else None
        first_done = bool(first_build and self.plan["tasks"].get(first_build["id"]) and all(t["completed_at"] or t["listed"] == "completed" for t in self.plan["tasks"][first_build["id"]]))
        after_doc = [p for p in self.own_pushes if self.doc_closed_at and p["at"] > self.doc_closed_at]
        self.write_state(
            doc_closed_at=self.doc_closed_at,
            own_pushes=len(self.own_pushes),
            own_pushes_after_doc=len(after_doc),
            first_build_group_done=first_done,
            live=list(live) if live else None,
            gates_open=[g["title"] for g in gs if not self.closed_row(g)],
            merged=bool(merged_g and self.closed_row(merged_g)),
            held=self.held,
        )

    def hold(self, g, reason):
        if self.held:
            self.log("hold_again", gate=g["gate"], reason=reason, already=self.held["gate"])
            return
        self.held = {"gate": g["gate"], "title": g["title"], "since": now(), "reason": reason}
        self.log("held", gate=g["gate"], title=g["title"], reason=reason)

    def refs_poll(self):
        rc, out = git("--git-dir", self.origin, "for-each-ref", "--format=%(refname:short) %(objectname)", "refs/heads")
        if rc != 0:
            return
        cur = dict(l.split(" ", 1) for l in out.splitlines() if " " in l)
        for ref, sha in cur.items():
            if self.refs.get(ref) != sha:
                own = ref == self.branch
                kind = "main_moved" if ref == "main" else ("push_own_branch" if own else "push_other_branch")
                n = git("--git-dir", self.origin, "rev-list", "--count", f"main..{sha}")[1] if ref != "main" else ""
                live = self.live if isinstance(self.live, tuple) else None
                self.log(kind, ref=ref, sha=sha[:10], previous=(self.refs.get(ref) or "")[:10], commits_past_main=n, live_group=live[1] if live else None)
                if own:
                    self.own_pushes.append({"at": now(), "sha": sha})
        self.refs = cur

    def run(self):
        self.log("keeper_started", branch=self.branch, template=[t for t in TEMPLATE_HEAD] + [TEMPLATE_TAIL])
        while True:
            stopping = os.path.exists(f"{self.run_dir}/gates.stop")
            try:
                self.poll(stopping)
            except Exception as e:  # the keeper must not die mid-run
                self.log("keeper_error", error=repr(e)[:300])
            if stopping:
                self.log("keeper_stopped", open=[g["title"] for g in (self.plan or {}).get("groups", []) if self.closed_row(g) is None], held=self.held)
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
    Keeper(ap.parse_args()).run()


if __name__ == "__main__":
    main()
