#!/usr/bin/env python3
"""The tracker for a Job that writes its own workflow. Stdlib only; run with `python3 -I`.

  track.py watch --scratch DIR --session ID --run-dir DIR --template TREE --rig DIR [--poll 2]
  track.py show  --run-dir DIR [--at ISO-TIME]

Started from docs/spikes/044-seeded/gates.py. There is no template. The plan the Job recorded with
record_plan is the workflow: its groups are the line, and the gate of a group is read from the end
of the group's `gives` ("... [gate: checks_green]"), because the record_plan route drops any field
it does not know (see the spike page). The first group's gate is always plan_approved.

Gates are Fleet's. Each is closed by a fact, in group order, forward only, and a closed gate is
never reopened or re-derived:

  plan_approved      session_plan.approved_at in the dev store
  walk_approved      a walk attachment of the Job in the ledger is approved       (not exercised)
  document_approved  a document attachment of the Job in the ledger is approved   (not exercised)
  checks_green       the repository's check for the touched files, run here at the commit the Job's
                     branch holds on the bare origin: vitest on each touched test file that exists
                     there. Evaluated only while this group is the live one, again at each new tip
  merged             the branch tip is an ancestor of origin's integration-<id8>
  none               the group ends when the next group starts: the first edit under the next group's
                     paths (not also under this group's), or the Job being done

The live group is the first group whose gate has not closed. Inside it the tracker records facts and
does not ask the agent: every Edit/Write the dev store kept a tool row for, placed against the groups'
paths; commits on the Job's branch in the clone; the branch tip on the bare origin (pushes). The agent's
task marks (the dev store's session_build_tasks) are logged separately as claims, with the live group
at that moment and the group the task belongs to.

A plan recorded again: closed gates are kept, the closed prefix is matched against the new groups by
gate kind, anything that does not match is moved to `orphaned` (kept, logged), and the live group is
recomputed.

Output, in <run-dir>: track.jsonl (every event), record.json (the state now). Ends when
<run-dir>/track.stop exists, after one last pass.
"""
import argparse
import json
import os
import re
import sqlite3
import subprocess
import sys
import time

KINDS = ("plan_approved", "walk_approved", "document_approved", "checks_green", "merged", "none")
TAG = re.compile(r"\[gate:\s*([A-Za-z_]+)\s*\]\s*$")
EDIT = re.compile(r"^(Edit|Write|MultiEdit|NotebookEdit)\s+(.*?)(?:\s[+-]\d+)*$")
SLOT = re.compile(r"\.armada/slots/slot-\d+/(.*)$")


def now():
    return time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime()) + ".%03dZ" % (int(time.time() * 1000) % 1000)


def git(*args, cwd=None):
    r = subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True)
    return r.returncode, r.stdout.strip()


def gate_of(group_index, gives):
    """(kind, note). The tag at the end of `gives`; the first group is plan_approved whatever it says."""
    m = TAG.search(gives or "")
    kind = m.group(1).lower() if m else None
    if group_index == 0:
        return "plan_approved", (None if kind == "plan_approved" else f"first group says {kind!r}, taken as plan_approved")
    if kind is None:
        return "none", "no gate tag, taken as none"
    if kind not in KINDS or kind == "plan_approved":
        return "none", f"unknown gate {kind!r}, taken as none"
    return kind, None


def under(rel, path):
    return rel == path or rel.startswith(path + "/")


def place(rel, groups, live):
    """How an edit sits against the plan. hits are the indexes of every group whose paths hold it."""
    hits = [i for i, g in enumerate(groups) if any(under(rel, p) for p in g["paths"])]
    if not groups:
        return "no_plan", hits
    if live in hits:
        return "in_live", hits
    if not hits:
        return "outside", hits
    return ("in_later" if any(i > live for i in hits) else "in_earlier"), hits


# ---------------------------------------------------------------------------------------------------------
def watch(a):
    sid, id8 = a.session, a.session[:8]
    home = f"{a.scratch}/home/user"
    db = f"{home}/Library/Application Support/Armada/armada.db"
    origin, repo = f"{a.scratch}/origin.git", f"{a.scratch}/repo"
    branch, target = f"armada/session-{id8}", f"integration-{id8}"
    check = f"{a.scratch}/verify-047-{id8}"
    out = f"{a.run_dir}/track.jsonl"

    S = {
        "plan_version": None, "groups": [], "closed": [], "orphaned": [], "live": 0,
        "placements": {}, "edits": [], "done": False, "tip": None, "commits": [],
    }

    def log(event, **kw):
        with open(out, "a") as f:
            f.write(json.dumps({"at": now(), "event": event, **kw}) + "\n")

    def save():
        tmp = f"{a.run_dir}/record.json.tmp"
        json.dump({"session": sid, **S}, open(tmp, "w"), indent=2)
        os.replace(tmp, f"{a.run_dir}/record.json")

    def con():
        c = sqlite3.connect(f"file:{db}?mode=ro", uri=True, timeout=10)
        c.row_factory = sqlite3.Row
        return c

    def live_title():
        return S["groups"][S["live"]]["title"] if S["live"] < len(S["groups"]) else None

    def relive():
        n = 0
        for g, c in zip(S["groups"], S["closed"]):
            if g["gate"] == c["kind"]:
                n += 1
            else:
                break
        S["live"] = n
        return n

    def close(kind, fact):
        idx = S["live"]
        S["closed"].append({"kind": kind, "closed_at": now(), "fact": fact, "group": S["groups"][idx]["title"], "group_index": idx, "plan_version": S["plan_version"]})
        S["live"] = idx + 1
        log("gate_closed", gate=kind, group_index=idx, group=S["groups"][idx]["title"], fact=fact)
        log("live_group", live=S["live"], group=live_title())
        save()

    # ---- the plan ---------------------------------------------------------------------------------------
    def read_plan():
        c = con()
        p = c.execute("select version, recorded_at, approved_at from session_plan where session_id=?", (sid,)).fetchone()
        if not p:
            c.close()
            return None
        groups = []
        for i, r in enumerate(c.execute("select group_id, title, paths, gives from session_plan_groups where session_id=? order by position", (sid,))):
            gate, note = gate_of(i, r["gives"])
            groups.append({"id": r["group_id"], "title": r["title"], "paths": [x for x in (r["paths"] or "").split("\n") if x], "gives": r["gives"], "gate": gate, "gate_note": note, "tasks": []})
        by_id = {g["id"]: g for g in groups}
        for r in c.execute("select group_id, task_id, text from session_plan_tasks where session_id=? order by position", (sid,)):
            if r["group_id"] in by_id:
                by_id[r["group_id"]]["tasks"].append({"id": r["task_id"], "text": r["text"]})
        c.close()
        return {"version": p["version"], "recorded_at": p["recorded_at"], "approved_at": p["approved_at"], "groups": groups}

    def take_plan(p):
        first = S["plan_version"] is None
        S["plan_version"], S["groups"] = p["version"], p["groups"]
        log("plan_recorded", version=p["version"], re_recorded=not first, groups=[{k: g[k] for k in ("title", "paths", "gate", "gate_note")} | {"tasks": len(g["tasks"])} for g in p["groups"]])
        before = len(S["closed"])
        kept = []
        for g, c in zip(S["groups"], S["closed"]):
            if g["gate"] == c["kind"]:
                kept.append(c)
            else:
                break
        if len(kept) < before:
            S["orphaned"] += S["closed"][len(kept):]
            S["closed"] = kept
            log("closed_gates_not_in_new_plan", kept=len(kept), orphaned=[c["kind"] for c in S["orphaned"]])
        relive()
        if not first:
            log("plan_remapped", closed_kept=[c["kind"] for c in S["closed"]], live=S["live"], group=live_title())
        else:
            log("live_group", live=S["live"], group=live_title())
        save()

    # ---- facts ------------------------------------------------------------------------------------------
    def tip():
        rc, o = git("--git-dir", origin, "rev-parse", "--verify", "-q", f"refs/heads/{branch}")
        return o if rc == 0 and o else None

    def touched(sha):
        rc, o = git("--git-dir", origin, "diff", "--name-only", f"main...{sha}")
        return o.splitlines() if rc == 0 else []

    def run_checks(sha):
        """The repository's check for the files this tip touched. Returns (green or None, what was said)."""
        files = [f for f in touched(sha) if re.search(r"\.test\.tsx?$", f) and f.startswith("apps/desktop/")]
        if not os.path.isdir(check):
            git("clone", "--quiet", "--local", origin, check)
            subprocess.run(["bash", f"{a.rig}/seed-node-modules.sh", a.template], cwd=check, capture_output=True)
        git("fetch", "--quiet", "origin", cwd=check)
        git("checkout", "--quiet", "--detach", "-f", sha, cwd=check)
        git("clean", "-fdq", cwd=check)
        files = [f for f in files if os.path.exists(f"{check}/{f}")]
        if not files:
            return None, "no touched test file exists at that commit, so no check applies"
        rel = [f[len("apps/desktop/"):] for f in files]
        t0 = time.time()
        r = subprocess.run(["node_modules/.bin/vitest", "run", *rel], cwd=f"{check}/apps/desktop", capture_output=True, text=True, timeout=600)
        text = r.stdout + r.stderr
        line = next((l.strip() for l in text.splitlines() if l.strip().startswith("Tests ")), "no Tests line")
        green = r.returncode == 0 and " passed" in line and " failed" not in line
        return green, f"vitest {' '.join(rel)}: {line} (exit {r.returncode}, {time.time() - t0:.0f}s)"

    def ledger_approved(c, kind):
        for r in c.execute("select target, state from ledger_attachment_versions where holder_id=? and kind=? order by at desc", (sid, kind)):
            if re.search(r"approv", r["state"] or "", re.I):
                return f"{kind} {r['target']} {r['state']}"
        return None

    checked_tip = None
    check_said_none = None

    def evaluate_live():
        nonlocal checked_tip, check_said_none
        while S["live"] < len(S["groups"]):
            g = S["groups"][S["live"]]
            kind, advanced = g["gate"], False
            if kind == "plan_approved":
                c = con()
                row = c.execute("select version, approved_at from session_plan where session_id=? and approved_at is not null", (sid,)).fetchone()
                c.close()
                if row:
                    close(kind, f"session_plan v{row[0]} approved_at {row[1]}")
                    advanced = True
            elif kind in ("walk_approved", "document_approved"):
                c = con()
                said = ledger_approved(c, kind.split("_")[0])
                c.close()
                if said:
                    close(kind, said)
                    advanced = True
            elif kind == "checks_green":
                sha = tip()
                if sha and sha != checked_tip:
                    checked_tip = sha
                    green, said = run_checks(sha)
                    log("checks_run", sha=sha[:10], green=green, said=said, group=g["title"])
                    if green:
                        close(kind, f"{said} at {sha[:10]}")
                        advanced = True
            elif kind == "merged":
                sha = tip()
                if sha:
                    rc, _ = git("--git-dir", origin, "merge-base", "--is-ancestor", sha, f"refs/heads/{target}")
                    if rc == 0:
                        close(kind, f"{sha[:10]} is an ancestor of origin/{target}")
                        advanced = True
            elif kind == "none":
                if S["done"]:
                    close(kind, "the Job is done, no later group started")
                    advanced = True
            if not advanced:
                return

    # ---- what the agent does ----------------------------------------------------------------------------
    last_seq = [0]

    def edits():
        c = con()
        rows = c.execute("select seq, body from session_rows where session_id=? and seq>? order by seq", (sid, last_seq[0])).fetchall()
        c.close()
        for r in rows:
            last_seq[0] = r["seq"]
            b = json.loads(r["body"])
            if b.get("kind") != "tool":
                continue
            m = EDIT.match(b.get("text", ""))
            if not m:
                continue
            path = m.group(2)
            sm = SLOT.search(path)
            rel = sm.group(1) if sm else path
            while S["groups"] and S["live"] < len(S["groups"]) and S["groups"][S["live"]]["gate"] == "none":
                _, hits = place(rel, S["groups"], S["live"])
                nxt = S["live"] + 1
                if nxt in hits and S["live"] not in hits:
                    close("none", f"first edit under the next group's paths: {rel}")
                    S["_started_next"] = True
                else:
                    break
            cls, hits = place(rel, S["groups"], S["live"])
            if S.pop("_started_next", False) and cls == "in_live":
                cls = "starts_next"
            S["placements"][cls] = S["placements"].get(cls, 0) + 1
            ev = {"path": rel, "tool": m.group(1), "placement": cls, "live": S["live"], "live_group": live_title(), "in_groups": hits, "row_at": b.get("at")}
            S["edits"].append(ev)
            log("edit", **ev)
            if cls in ("in_later", "outside", "no_plan", "in_earlier"):
                log("off_group", path=rel, placement=cls, live_group=live_title(), in_groups=[S["groups"][i]["title"] for i in hits])
        save()

    seen_commits = set()

    def commits():
        rc, o = git("rev-list", "--reverse", f"refs/heads/main..refs/heads/{branch}", cwd=repo)
        if rc != 0:
            return
        for sha in o.splitlines():
            if sha not in seen_commits:
                seen_commits.add(sha)
                _, subj = git("log", "-1", "--format=%s", sha, cwd=repo)
                _, files = git("diff-tree", "--no-commit-id", "--name-only", "-r", sha, cwd=repo)
                S["commits"].append({"sha": sha[:10], "live": S["live"]})
                log("commit", sha=sha[:10], subject=subj, files=files.splitlines(), live_group=live_title(),
                    placements=[place(f, S["groups"], S["live"])[0] for f in files.splitlines()])

    def push():
        sha = tip()
        if sha and sha != S["tip"]:
            S["tip"] = sha
            log("push", sha=sha[:10], live_group=live_title())
            save()

    task_seen = {}

    def claims():
        c = con()
        rows = c.execute("select task_id, text, status from session_build_tasks where session_id=? order by position", (sid,)).fetchall()
        c.close()
        for r in rows:
            if task_seen.get(r["task_id"]) == r["status"]:
                continue
            first = r["task_id"] not in task_seen
            task_seen[r["task_id"]] = r["status"]
            owner = next((i for i, g in enumerate(S["groups"]) for t in g["tasks"] if t["text"].strip() == r["text"].strip()), None)
            group_edits = sum(1 for e in S["edits"] if owner is not None and owner in e["in_groups"])
            flag = None
            if owner is not None and owner > S["live"] and r["status"] != "pending":
                flag = "ahead_of_live_group"
            elif r["status"] == "completed" and owner is not None and S["groups"][owner]["paths"] and group_edits == 0:
                flag = "completed_with_no_edit_in_its_group_paths"
            log("claim", task=r["task_id"], text=r["text"][:90], status=r["status"], created=first, belongs_to=(S["groups"][owner]["title"] if owner is not None else None),
                live_group=live_title(), edits_in_its_group_so_far=group_edits, flag=flag)

    def done():
        c = con()
        row = c.execute("select since from session_done where session_id=?", (sid,)).fetchone()
        c.close()
        if row and not S["done"]:
            S["done"] = True
            log("job_done", since=row["since"], live_group=live_title())

    log("tracker_started", branch=branch, target=target)
    save()
    while True:
        stopping = os.path.exists(f"{a.run_dir}/track.stop")
        try:
            p = read_plan()
            if p and (S["plan_version"] != p["version"]):
                take_plan(p)
            if p and p["groups"]:
                evaluate_live()
            edits()
            commits()
            push()
            claims()
            done()
            if p and p["groups"]:
                evaluate_live()
        except sqlite3.Error as e:
            log("store_read_failed", error=str(e)[:100])
        if stopping:
            log("tracker_stopped", open=[g["title"] for g in S["groups"][S["live"]:]], live=S["live"])
            save()
            return
        time.sleep(a.poll)


# ---------------------------------------------------------------------------------------------------------
def show(a):
    """The progress display rebuilt from track.jsonl, as of --at (default: the end)."""
    ev = [json.loads(l) for l in open(f"{a.run_dir}/track.jsonl")]
    if a.at:
        ev = [e for e in ev if e["at"] <= a.at]
    groups, closed, live, edits, commits, pushes, claim, done = [], {}, 0, {}, {}, 0, None, False
    for e in ev:
        k = e["event"]
        if k == "plan_recorded":
            groups = e["groups"]
        elif k == "gate_closed":
            closed[e["group_index"]] = e
        elif k == "live_group":
            live = e["live"]
        elif k == "plan_remapped":
            live = e["live"]
        elif k == "edit":
            for i in e["in_groups"] or []:
                edits[i] = edits.get(i, 0) + 1
            if not e["in_groups"]:
                edits["outside"] = edits.get("outside", 0) + 1
        elif k == "commit":
            commits[live] = commits.get(live, 0) + 1
        elif k == "push":
            pushes += 1
        elif k == "claim" and e["status"] == "in_progress":
            claim = e["text"]
        elif k == "job_done":
            done = True
    print(f"as of {ev[-1]['at'] if ev else '-'}")
    for i, g in enumerate(groups):
        state = f"closed {closed[i]['at'][11:19]}" if i in closed else ("OPEN, live" if i == live else "open")
        print(f"  {i + 1}. {g['title'][:44]:44}  gate {g['gate']:16} {state}")
        if i == live and i not in closed:
            print(f"       activity: {edits.get(i, 0)} edits under its paths, {commits.get(i, 0)} commits, {pushes} pushes so far; outside every group: {edits.get('outside', 0)}")
            print(f"       agent says: {claim or '-'}")
    if done:
        print("  Job done")


def main():
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)
    w = sub.add_parser("watch")
    for n in ("scratch", "session", "run-dir", "template", "rig"):
        w.add_argument("--" + n, required=True)
    w.add_argument("--poll", type=float, default=2)
    s = sub.add_parser("show")
    s.add_argument("--run-dir", required=True)
    s.add_argument("--at")
    a = ap.parse_args()
    {"watch": watch, "show": show}[a.cmd](a)


if __name__ == "__main__":
    main()
