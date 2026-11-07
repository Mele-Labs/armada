#!/usr/bin/env python3
"""The gate keeper: Fleet's side of the seeded workflow. Stdlib only; run with `python3 -I`.

  gates.py --scratch DIR --session ID --run-dir DIR --test-file REL --template TREE [--poll 2]

Three gates, forward only. A gate that has closed is never re-derived and never reopened.

  plan_approved   the dev store's session_plan row for the Job has approved_at
  checks_green    the yardstick's one vitest file, run here at the commit the Job's branch holds on
                  the bare origin, in a scratch checkout, exits 0 with at least one test passed.
                  Evaluated only once plan_approved has closed, and again each time the tip moves
                  until it is green.
  merged          the branch's tip is an ancestor of origin's integration-<id8> (what the person's
                  merge pushes). Evaluated only once checks_green has closed.

On a close it marks the gate's task completed in the Job's own task list, with an atomic rename.
A gate task the agent marks completed before its gate closed is put back to pending (a claim is
never a gate), and both are logged. Everything goes to <run-dir>/gates.jsonl and the record to
<run-dir>/record.json: each gate's closed_at and the fact that closed it.

Ends when <run-dir>/gates.stop exists (run.sh writes it once the person is done), after one last pass.
"""
import argparse
import json
import os
import sqlite3
import subprocess
import time


def now():
    return time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime()) + ".%03dZ" % (int(time.time() * 1000) % 1000)


def git(*args, cwd=None):
    r = subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True)
    return r.returncode, r.stdout.strip()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--scratch", required=True)
    ap.add_argument("--session", required=True)
    ap.add_argument("--run-dir", required=True)
    ap.add_argument("--test-file", required=True, help="relative to apps/desktop")
    ap.add_argument("--template", required=True, help="a tree with node_modules to copy from")
    ap.add_argument("--rig", required=True, help="the 042-rig directory (seed-node-modules.sh)")
    ap.add_argument("--poll", type=float, default=2)
    a = ap.parse_args()

    sid, id8 = a.session, a.session[:8]
    home = f"{a.scratch}/home/user"
    db = f"{home}/Library/Application Support/Armada/armada.db"
    origin = f"{a.scratch}/origin.git"
    branch, target = f"armada/session-{id8}", f"integration-{id8}"
    meta = json.load(open(f"{a.run_dir}/template.json"))
    tdir, tasknos = meta["dir"], meta["gates"]
    check = f"{a.scratch}/verify-044-{id8}"
    out = f"{a.run_dir}/gates.jsonl"

    gates = {
        "plan_approved": {"name": "the person approved the plan", "task": tasknos["plan_approved"], "closed_at": None, "fact": None},
        "checks_green": {"name": "Checks green at the pushed commit", "task": tasknos["checks_green"], "closed_at": None, "fact": None},
        "merged": {"name": "the branch merged", "task": tasknos["merged"], "closed_at": None, "fact": None},
    }
    order = ["plan_approved", "checks_green", "merged"]
    gate_of_task = {g["task"]: k for k, g in gates.items()}

    def log(event, **kw):
        with open(out, "a") as f:
            f.write(json.dumps({"at": now(), "event": event, **kw}) + "\n")

    def save():
        tmp = f"{a.run_dir}/record.json.tmp"
        json.dump({"session": sid, "gates": gates}, open(tmp, "w"), indent=2)
        os.replace(tmp, f"{a.run_dir}/record.json")

    def read_task(n):
        try:
            return json.load(open(f"{tdir}/{n}.json"))
        except (OSError, ValueError):
            return None

    def mark(n, status):
        t = read_task(n)
        if t is None or t.get("status") == status:
            return False
        t["status"] = status
        tmp = f"{tdir}/.{n}.json.fleet"
        json.dump(t, open(tmp, "w"), indent=2)
        os.replace(tmp, f"{tdir}/{n}.json")
        return True

    def close(key, fact):
        g = gates[key]
        g["closed_at"], g["fact"] = now(), fact
        ticked = mark(g["task"], "completed")
        log("gate_closed", gate=key, fact=fact, ticked_task=g["task"] if ticked else None)
        save()

    def tip():
        rc, out_ = git("--git-dir", origin, "rev-parse", "--verify", "-q", f"refs/heads/{branch}")
        return out_ if rc == 0 and out_ else None

    def run_checks(sha):
        if not os.path.isdir(check):
            git("clone", "--quiet", "--local", origin, check)
            subprocess.run(["bash", f"{a.rig}/seed-node-modules.sh", a.template], cwd=check, capture_output=True)
        git("fetch", "--quiet", "origin", cwd=check)
        git("checkout", "--quiet", "--detach", "-f", sha, cwd=check)
        git("clean", "-fdq", cwd=check)
        d = f"{check}/apps/desktop"
        if not os.path.exists(f"{d}/{a.test_file}"):
            return False, "test file does not exist at that commit"
        t0 = time.time()
        r = subprocess.run(["node_modules/.bin/vitest", "run", a.test_file], cwd=d, capture_output=True, text=True, timeout=600)
        text = r.stdout + r.stderr
        line = next((l.strip() for l in text.splitlines() if l.strip().startswith("Tests ")), "no Tests line")
        green = r.returncode == 0 and " passed" in line and " failed" not in line
        return green, f"vitest {a.test_file}: {line} (exit {r.returncode}, {time.time() - t0:.0f}s)"

    seen_status = {}  # task file status as last read
    checked_tip = None
    log("keeper_started", branch=branch, target=target, test_file=a.test_file)
    save()

    stopping = False
    while True:
        stopping = os.path.exists(f"{a.run_dir}/gates.stop")

        # --- what the agent marks in the task list: its claim; a claim on a gate is put back -------------
        try:
            names = sorted(n for n in os.listdir(tdir) if n.endswith(".json") and not n.startswith("."))
        except OSError:
            names = []
        for name in names:
            n = name[:-5]
            t = read_task(n)
            if t is None:
                continue
            status = t.get("status")
            if seen_status.get(n) != status:
                first = n not in seen_status
                if n in gate_of_task:
                    key = gate_of_task[n]
                    if gates[key]["closed_at"] is None and status != "pending":
                        log("claim_on_gate", gate=key, task=n, claimed=status, subject=t.get("subject"))
                        if status == "completed":
                            mark(n, "pending")
                            log("claim_put_back", gate=key, task=n)
                            status = "pending"
                elif not (first and status == "pending"):
                    log("task_status", task=n, status=status, subject=t.get("subject"))
                seen_status[n] = status

        # --- gate 1 -------------------------------------------------------------------------------------
        if gates["plan_approved"]["closed_at"] is None:
            try:
                con = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
                row = con.execute("select version, approved_at from session_plan where session_id=? and approved_at is not null", (sid,)).fetchone()
                con.close()
            except sqlite3.Error as e:
                row = None
                log("store_read_failed", error=str(e)[:100])
            if row:
                close("plan_approved", f"session_plan v{row[0]} approved_at {row[1]}")

        # --- gate 2 -------------------------------------------------------------------------------------
        if gates["plan_approved"]["closed_at"] and gates["checks_green"]["closed_at"] is None:
            sha = tip()
            if sha and sha != checked_tip:
                checked_tip = sha
                green, said = run_checks(sha)
                log("checks_run", sha=sha[:10], green=green, said=said)
                if green:
                    close("checks_green", f"{said} at {sha[:10]}")
        # a push before the plan was approved is noted once, not used
        elif gates["plan_approved"]["closed_at"] is None and tip() and checked_tip is None:
            checked_tip = "noted"
            log("pushed_before_plan_approved", sha=tip()[:10])

        # --- gate 3 -------------------------------------------------------------------------------------
        if gates["checks_green"]["closed_at"] and gates["merged"]["closed_at"] is None:
            sha = tip()
            if sha:
                rc, _ = git("--git-dir", origin, "merge-base", "--is-ancestor", sha, f"refs/heads/{target}")
                if rc == 0:
                    close("merged", f"{sha[:10]} is an ancestor of origin/{target}")

        if stopping:
            log("keeper_stopped", open=[k for k in order if gates[k]["closed_at"] is None])
            save()
            return
        time.sleep(a.poll)


if __name__ == "__main__":
    main()
