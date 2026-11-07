#!/usr/bin/env python3
"""Spike 041: replay Fleet's rail disputes against a gate-only placement.

Run from the repository root:  python3 -I docs/spikes/041-replay.py > docs/spikes/041-output.txt

Reads the owner's store read-only and `git log` of .armada/workflows. Writes nothing.
Question and method: docs/spikes/041-gate-only-placement.md.
"""
import collections
import datetime
import json
import os
import random
import re
import sqlite3
import subprocess
import sys

DB = os.path.expanduser("~/Library/Application Support/Armada/armada.db")
con = sqlite3.connect("file:" + DB + "?mode=ro", uri=True)
WF_FILES = {"bug": "bug", "feature": "feature", "refactor": "refactor", "orchestrate": "orchestrate",
            "design_plan": "design-plan", "prototype": "prototype", "revert": "revert", "uat": "uat",
            "code_review": "code-review"}
DONE = "done"
JOBS = {}


def git(*a):
    return subprocess.run(["git", *a], capture_output=True, text=True).stdout


def load_json(text):
    return json.loads("".join(l for l in text.splitlines(True) if not l.lstrip().startswith("#")))


# ---------------------------------------------------------------- workflows

def norm_step(s):
    ev = s.get("evidence") or {}
    sub = (ev.get("submitted") if "submitted" in ev else s.get("submitted")) or {}
    etype = sub.get("type")
    ends = [e for e in (s.get("ends_on") or [])]
    if s.get("awaits_reply") and "replied" not in ends:
        ends = ["replied"] + ends
    mech = [m.get("type") for m in s.get("mechanical_checks", [])]
    st = dict(id=s["id"], label=s.get("label", s["id"]), records_plan=bool(s.get("records_plan")) or etype == "plan",
              plan_unasked=bool(s.get("plan_unasked")), follows_plan=bool(s.get("follows_plan")),
              delivers=bool(s.get("delivers")), walks=bool(s.get("walks")), ends_on=ends, etype=etype,
              manifest_check="every_manifest_check" in mech, phase=s.get("phase"))
    st["fit"] = None
    if not (st["records_plan"] or st["follows_plan"] or st["delivers"]):
        if etype == "diff":
            st["fit"] = "edited"
        elif etype in ("test_suite_run", "failing_test") or st["manifest_check"]:
            st["fit"] = "checked"
        elif st["phase"] == "delivery":
            st["fit"] = "committed"
        elif etype == "review":
            st["fit"] = "pr_opened"
    return st


def versions(name):
    """Every version of a shipped workflow since 4 Oct, as (commit date, steps)."""
    path = f".armada/workflows/{WF_FILES[name]}.json"
    out = []
    for line in git("log", "--format=%h %aI", "--since=2026-10-04", "--reverse", "--", path).splitlines():
        h, d = line.split()
        d = datetime.datetime.fromisoformat(d).astimezone(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.000Z")
        try:
            w = load_json(git("show", f"{h}:{path}"))
            out.append((d, [norm_step(s) for s in w["steps"]]))
        except Exception:
            pass
    return out


VERSIONS = {n: versions(n) for n in WF_FILES}


def pick_version(name, labels, when):
    """The version in force at `when` whose labels hold every label the Job's rail named; else the nearest that does."""
    vs = VERSIONS.get(name) or []
    ok = [(d, s) for d, s in vs if set(labels) <= {x["label"] for x in s}]
    if not ok:
        return None
    before = [v for v in ok if v[0] <= when]
    return (before[-1] if before else ok[0])[1]


def recorder(steps):
    return next((i for i, s in enumerate(steps) if s["records_plan"]), None)


def ends_of(steps, i):
    """Port of rail_ends::ends_of. Each end is a kind string."""
    s = steps[i]
    rec = recorder(steps)
    listed = lambda: [e for e in s["ends_on"]]
    if s["delivers"]:
        return ["merged"]
    if s["records_plan"] and not s["plan_unasked"]:
        return ["plan_approved"] + listed()
    if s["records_plan"] and not s["follows_plan"]:
        return ["plan_recorded"]
    if s["walks"]:
        return ["walk_approved_all"]
    if any(e not in ("replied", "walk_approved", "sketch_approved", "draft_approved", "idea_picked",
                     "document_approved") for e in s["ends_on"]):
        return listed()  # lifecycle
    if rec is not None and i < rec:
        return listed() if s["ends_on"] else ["plan_recorded"]
    if i + 1 < len(steps):
        n = steps[i + 1]
        if n["delivers"]:
            return ["pushed"]
        if n["walks"]:
            return ["walk_shown"]
        if n["fit"]:
            return [n["fit"]]
    return []


PERSON = {"plan_approved", "walk_approved", "walk_approved_all", "sketch_approved", "draft_approved",
          "idea_picked", "document_approved"}
PR = {"merged", "pr_opened"}
STRICT_GATES = PERSON | PR | {"checked"}  # "checked" reads as Checks green
NOT_GATES = {"plan_recorded", "replied", "walk_shown", "edited", "committed", "pushed"}


def role_of(steps, i):
    """The roles a step plays, as a set: plan, walk, build, tests, deliver, or done past the last step."""
    if i >= len(steps):
        return {DONE}
    s = steps[i]
    rec = recorder(steps)
    if rec is not None and i <= rec and not s["follows_plan"]:
        return {"plan"}
    roles = set()
    if s["walks"]:
        roles.add("walk")
    if s["delivers"]:
        roles.add("deliver")
    if re.search(r"test", s["label"], re.I):
        roles.add("tests")
    if s["follows_plan"] or s["etype"] == "diff":
        roles.add("build")
    return roles or {"other:" + s["label"]}


def fits(t, roles):
    """Whether a step playing `roles` is where the reason puts the Job. Tests are part of building."""
    return t in roles or (t in ("build", "tests") and bool(roles & {"build", "tests"}))


def true_roles(steps, t):
    """The agent's reason names a role; a workflow without that role means the nearest one it has."""
    if t == "walk" and not any(s["walks"] for s in steps):
        return "build"
    if t == "tests" and not any(re.search(r"test", s["label"], re.I) for s in steps):
        return "build"
    return t


# ---------------------------------------------------------------- facts of a Job

def page_of(url):
    m = re.match(r"([a-z]+://)?([^/:?#]+)(:\d+)?(.*)", url)
    return (m.group(2) + m.group(4).rstrip("/")) if m else url


def approved_address(text):
    t = text.strip()
    if not t.startswith("Approved") or t.startswith("Approved sketch: "):
        return None
    rest = t[len("Approved"):]
    if rest.startswith(": "):
        return rest[2:].strip()
    m = re.match(r" [A-Z], .*: (\S+)$", rest, re.S)
    return m.group(1) if m else None


class Facts:
    def __init__(self, sid):
        self.sid = sid
        rows = [json.loads(b) for (b,) in con.execute("select body from session_rows where session_id=? order by seq", (sid,))]
        you = lambda r: r.get("kind") == "message" and (r.get("from") or {}).get("kind") == "you"
        self.windows = [(r["at"], r["url"]) for r in rows if r.get("kind") == "window"]
        self.walk_ok = [(r["at"], a) for r in rows if you(r) and (a := approved_address(r["text"]))]
        self.sketch_ok = [r["at"] for r in rows if you(r) and r["text"].startswith("Approved sketch: ")]
        self.draft_ok = [r["settled"]["at"] for r in rows if r.get("kind") == "draft"
                         and (r.get("settled") or {}).get("how") in ("approved", "fixed")]
        asks = {r["id"]: r for r in rows if r.get("kind") == "ask"}
        self.idea_ok = []
        for r in rows:
            if you(r) and r["id"].startswith("ask-") and r["id"].endswith(":said"):
                ask = asks.get(r["id"][: -len(":said")])
                qs = ((ask or {}).get("ask") or {}).get("questions") or []
                if any(o.get("preview") and o["label"] in r["text"] for q in qs for o in q.get("options", [])):
                    self.idea_ok.append(r["at"])
        p = con.execute("select recorded_at, approved_at, version from session_plan where session_id=?", (sid,)).fetchone()
        self.plan_rec, self.plan_ok = (p or (None, None))[:2]
        # Every plan version recorded, and every approval of one: the store keeps only the newest version's approval,
        # so the earlier ones are read off Fleet's own line in the thread, "The owner approved plan vN."
        self.plan_versions = [(r["at"], r["version"]) for r in rows if r.get("kind") == "plan"]
        self.plan_approvals = [(r["at"], int(m.group(1))) for r in rows
                               if r.get("kind") == "message" and (r.get("from") or {}).get("id") == "fleet"
                               and (m := re.match(r"The owner approved plan v(\d+)\.", r.get("text", "")))]
        if p and p[1] and p[2] and not any(v == p[2] for _, v in self.plan_approvals):
            self.plan_approvals.append((p[1], p[2]))
        self.doc_ok, prs, self.branches = [], [], set()
        for kind, target, manifest, detail in con.execute(
                "select kind, target, manifest_id, detail from ledger_attachments where holder_kind='session' and holder_id=?", (sid,)):
            d = json.loads(detail or "{}")
            if kind == "artifact" and d.get("settled") == "approved" and d.get("settled_at"):
                self.doc_ok.append(d["settled_at"])
            if kind == "pr":
                prs.append((manifest, target))
                if d.get("branch"):
                    self.branches.add(d["branch"])  # the head branch of the Job's own pull request
            if kind == "branch" and sid[:8] in target:
                self.branches.add(target)  # the Job's own branch is named for it; the ledger also lists subagents' branches
        ev = lambda k, pr: [r[0] for r in con.execute(
            "select at from timeline_events where subject_kind='pull_request' and subject_id=? and repo=? and kind=? order by at", (pr[1], pr[0], k))]
        self.pr_open = sorted(t for pr in prs for t in ev("pr.opened", pr)[:1])
        self.pr_merge = sorted(t for pr in prs for t in ev("pr.merged", pr)[:1])
        self.pr_green = sorted(r[0] for pr in prs for r in con.execute(
            "select at from timeline_events where subject_kind='pull_request' and subject_id=? and repo=? and kind='pr.checks' and says like 'Checks passed%'", (pr[1], pr[0])))
        self.pushes = [(a, b) for a, b in con.execute(
            "select at, detail from timeline_events where subject_kind='session' and subject_id=? and kind='session.pushed' order by at", (sid,))]
        self.runs = con.execute(
            "select ended_at, name, state from session_check_runs where session_id=? and ended_at is not null order by ended_at", (sid,)).fetchall()
        self.steps_ev = []
        for at, says in con.execute("select at, says from timeline_events where subject_kind='session' and subject_id=? and kind='session.step' order by at, seq", (sid,)):
            m = re.match(r"step: (.*) → (.*)$", says)
            if m:
                self.steps_ev.append((at, m.group(2)))

    # --- gates closed at t
    def first(self, xs, t):
        return any(x <= t for x in xs)

    def plan_approved(self, t):
        """The newest plan version recorded by t has been approved by t. A plan recorded again waits for him again."""
        recorded = [v for at, v in self.plan_versions if at <= t]
        newest = max(recorded) if recorded else None
        return any(at <= t and (newest is None or v >= newest) for at, v in self.plan_approvals)

    def plan_approved_since(self, t):
        """When the plan standing at t was approved, for the walks shown since."""
        recorded = [v for at, v in self.plan_versions if at <= t]
        newest = max(recorded) if recorded else None
        xs = [at for at, v in self.plan_approvals if at <= t and (newest is None or v >= newest)]
        return min(xs) if xs else None

    def any_walk_approved(self, t):
        return any(a <= t for a, _ in self.walk_ok)

    def walks_all_approved(self, t):
        """Every window shown since the plan's approval stands approved (newest of each page), at least one."""
        since = self.plan_approved_since(t)
        if since is None:
            return False
        newest = {}
        for at, url in self.windows:
            if since <= at <= t:
                newest[page_of(url)] = (at, url)
        if not newest:
            return False
        for at, url in newest.values():
            if not any(a >= at and a <= t and page_of(ad) == page_of(url) for a, ad in self.walk_ok):
                return False
        return True

    def last_walk_ok(self, t):
        xs = [a for a, _ in self.walk_ok if a <= t]
        return max(xs) if xs else None

    def checks_green(self, t):
        if self.first(self.pr_green, t):
            return True
        latest = {}
        for at, name, state in self.runs:
            if at <= t:
                latest[name] = state
        return bool(latest) and all(s == "passed" for s in latest.values())

    def own_push(self, t, after=None):
        for at, branch in self.pushes:
            if at > t:
                break
            if branch and self.branches and branch not in self.branches:
                continue  # a push to a branch that is not the Job's own (a subagent's)
            if after is None or at >= after:
                return True
        return False

    def old_step(self, t):
        cur = None
        for at, label in self.steps_ev:
            if at <= t:
                cur = label
        return cur


def closed(f, steps, i, t, variant):
    """Whether the end of step i is a gate Fleet saw close by t, under `variant` (S, P)."""
    for e in ends_of(steps, i):
        if e == "plan_approved" and f.plan_approved(t):
            return True
        if e == "walk_approved_all" and f.walks_all_approved(t):
            return True
        if e == "walk_approved" and f.any_walk_approved(t):
            return True
        if e == "sketch_approved" and f.first(f.sketch_ok, t):
            return True
        if e == "draft_approved" and f.first(f.draft_ok, t):
            return True
        if e == "idea_picked" and f.first(f.idea_ok, t):
            return True
        if e == "document_approved" and f.first(f.doc_ok, t):
            return True
        if e == "merged" and f.first(f.pr_merge, t):
            return True
        if e == "pr_opened" and f.first(f.pr_open, t):
            return True
        if e == "checked" and f.checks_green(t):
            return True
        if e == "pushed" and variant in ("P", "M"):
            walls = any(s["walks"] for s in steps[:i])
            if walls:  # a push made before his Approve was the walk's own, and none counts while he has not approved
                after = f.last_walk_ok(t)
                if after is not None and f.own_push(t, after=after):
                    return True
            elif f.own_push(t):
                return True
    return False


def gate_place(f, steps, t, variant):
    """The step index gate-only placement holds the Job on at t (len(steps) is done)."""
    ok = [closed(f, steps, i, t, variant) for i in range(len(steps))]
    if variant == "M":  # a gate closed on a later step, or entering delivery, passes every step before it but a walk
        last = max([i for i, c in enumerate(ok) if c] + [-1])
        d = next((i for i, s in enumerate(steps) if s["delivers"]), None)
        if d is not None and f.first(f.pr_open, t):
            last = max(last, d)  # a pull request opened: delivery is entered, so every step before it is passed
        ok = [c or (i < last and not steps[i]["walks"]) for i, c in enumerate(ok)]
    for i, c in enumerate(ok):
        if not c:
            return i
    return len(steps)


def old_place(f, steps, t):
    lab = f.old_step(t)
    if lab is None:
        return 0
    if lab == "done":
        return len(steps)
    return next((i for i, s in enumerate(steps) if s["label"] == lab), 0)


# ---------------------------------------------------------------- the disputes

def kind_of(n):
    if n.startswith("Fleet set"):
        return "tasks"
    if "you are on its" in n:
        return "on-step"
    if "pushed while tasks" in n:
        return "push-open"
    if "edited files before" in n:
        return "edit-before-plan"
    if "edited a test" in n:
        return "test-before-walk"
    if "turn ended" in n:
        return "turn-end"
    return "other"


def named_label(n):
    m = re.search(r'its "(.*?)" step', n)
    return m.group(1) if m else None


def workflow_name(sid, notice):
    m = re.search(r"follows the (\w+) workflow", notice)
    if m:
        return m.group(1)
    r = con.execute("select workflow from session_started where session_id=?", (sid,)).fetchone()
    return r[0] if r and r[0] else None


def truth_role(kind, why):
    """The automatic judge, second version: where the agent's reason says the Job really was, as a role.
    None when the reason names no step (a task mark, an owner's stop, an instruction disputed)."""
    t = why.lower()
    merged = re.search(r"merged (into main|as #\d+)|#\d+( has)?( already)? merged|has (already )?merged|already merged|finish_up|landed as|is landed|were merged|pull requests? .{0,40}merged", t)
    open_pr = re.search(r"(pull request|pr) #?\d+ (is )?(open|queued)|#\d+ (is )?(open|queued)|auto-merge|merge queue|waiting on (its|the) merge", t)
    unwalked = re.search(r"(not|never|hasn't|n't) (yet )?(been )?(approved|walked)|unapproved|just (shown|showed)|only just|awaits? (the owner's |his )?approval|withdrew|has not approved|never approved|walk (is )?not (yet )?(shown|approved)|walk has not|walked nothing|has not ended|no walk was shown|must walk", t)
    walked = re.search(r"walk[^.]{0,80}approved (before|earlier)|approved the (mock |short )?walk|walk[^.]{0,40}had already passed|walk[^.]{0,60}(shown and )?approved before", t)
    unfinished = re.search(r"not finished|isn't finished|is not finished|still (being )?(built|building|implementing|running|in progress)|tasks? .{0,30}(are|is) (still )?open|are still open|not ready|premature|too early|not yet|still on|not done|not at deliver", t)
    again = re.search(r"v\d+ (is |was )?(now )?(recorded|waiting)|recording v\d|re-?recording|recorded v\d|moved the (step|job) back|went back to plan|waits? on his approval", t)
    plan_wait = re.search(r"plan v\d+ is recorded|waits? on his approval|not approved the plan|draft step|on that step|outside the (worktree|repository)|memory note|no repository file", t)
    plan_ok = re.search(r"(plan|v\d)[^.]{0,30}approved|approved plan|owner approved", t)
    if kind == "tasks":
        return None
    if kind == "edit-before-plan":
        if (plan_wait or again) and not (plan_ok and not again):
            return "plan"
        return "build" if plan_ok else None
    if kind == "test-before-walk":
        return "build" if re.search(r"approved", t) else None
    if merged and not unfinished and not open_pr:
        return DONE
    if open_pr and not unfinished:
        return "deliver"
    if unwalked and not walked:
        return "walk"
    if unfinished:
        return "build"
    if walked:
        return "build"
    if plan_wait:
        return "plan"
    if plan_ok and kind == "push-open":
        return "build"
    return None


def truth_role_v1(kind, why):
    """The first version of the judge, kept to report how it did on the first 30."""
    t = why.lower()
    merged = re.search(r"merged (into main|as #\d+)|#\d+( has)?( already)? merged|has (already )?merged|already merged|finish_up|landed as|is landed|were merged|pull requests? .{0,40}merged", t)
    open_pr = re.search(r"(pull request|pr) #?\d+ (is )?(open|queued)|#\d+ (is )?(open|queued)|auto-merge|merge queue|waiting on (its|the) merge", t)
    unwalked = re.search(r"(not|never|hasn't|n't) (yet )?(been )?(approved|walked)|unapproved|just (shown|showed)|only just|awaits? (the owner's |his )?approval|withdrew|has not approved|never approved|walk (is )?not (yet )?(shown|approved)|walk has not|walked nothing", t)
    unfinished = re.search(r"not finished|isn't finished|is not finished|still (being )?(built|building|implementing|running|in progress)|tasks? .{0,30}(are|is) (still )?open|are still open|not ready|premature|too early|not yet|still on|not done|not at deliver", t)
    plan_wait = re.search(r"plan v\d+ is recorded|waits? on his approval|not approved the plan|draft step|on that step|outside the (worktree|repository)|memory note|no repository file", t)
    plan_ok = re.search(r"(plan|v\d)[^.]{0,30}approved|approved plan|owner approved", t)
    if kind == "tasks":
        return None
    if kind == "edit-before-plan":
        if plan_wait and not plan_ok:
            return "plan"
        return "build" if plan_ok else None
    if kind == "test-before-walk":
        return "tests" if re.search(r"approved", t) else None
    if merged and not unfinished and not open_pr:
        return DONE
    if open_pr and not unfinished:
        return "deliver"
    if unwalked:
        return "walk"
    if unfinished:
        return "build"
    if plan_wait:
        return "plan"
    if plan_ok and kind == "push-open":
        return "build"
    return None


def load_disputes():
    out = []
    q = """select d.session_id, d.notice_row, d.notice, d.why,
           (select json_extract(body,'$.at') from session_rows r where r.row_id=d.notice_row limit 1)
           from fleet_disputes d where notice like 'Fleet: this Job follows%' or notice like 'Fleet set these tasks%' order by d.at"""
    for sid, row, notice, why, sent in con.execute(q):
        out.append(dict(sid=sid, row=row, notice=notice, why=why, t=sent, kind=kind_of(notice), label=named_label(notice)))
    return out


def prepare(sid, wf, labels, started):
    steps = pick_version(wf, labels, started) if wf in VERSIONS else None
    return steps


def main():
    jobs = {}  # sid -> dict(facts, steps, wf)
    all_sids = [r[0] for r in con.execute("select distinct subject_id from timeline_events where kind='session.step'")]
    disputes = load_disputes()
    wf_by_dispute = {}
    for d in disputes:
        if d["notice"].startswith("Fleet: this Job follows"):
            wf_by_dispute.setdefault(d["sid"], workflow_name(d["sid"], d["notice"]))
    for sid in all_sids:
        f = Facts(sid)
        if not f.steps_ev:
            continue
        wf = wf_by_dispute.get(sid) or workflow_name(sid, "")
        started, ended = con.execute("select started_at, coalesce(ended_at,last_seen_at) from sessions where id=?", (sid,)).fetchone() or ("", "")
        labels = {l for _, l in f.steps_ev if l != "done"}
        steps = pick_version(wf, labels, started) if wf in VERSIONS else None
        if steps is None and not wf:
            for cand in VERSIONS:  # infer the workflow from the labels the rail named
                steps = pick_version(cand, labels, started)
                if steps:
                    wf = cand
                    break
        jobs[sid] = dict(f=f, steps=steps, wf=wf, started=started, ended=ended)
    print("== Population")
    print(f"jobs with a rail history (session.step events): {len(jobs)}")
    print(f"  workflow resolved to a version containing every label the rail named: {sum(1 for j in jobs.values() if j['steps'])}")
    print("  by workflow:", dict(collections.Counter(j['wf'] for j in jobs.values() if j['steps'])))
    rail_ids = {r[0] for r in con.execute("select session_id from session_rail where visits<>''")}
    print(f"  of which have session_rail.visits (the brief's 165): {len(rail_ids & set(jobs))}; the other {len(jobs)-len(rail_ids & set(jobs))} have an empty visits column but the same entries on the timeline")
    agree = total = 0
    for sid in rail_ids & set(jobs):
        j = jobs[sid]
        if not j["steps"]:
            continue
        vis = con.execute("select visits from session_rail where session_id=?", (sid,)).fetchone()[0]
        got = [j["steps"][int(l.split("\t")[0])]["label"] for l in vis.splitlines() if int(l.split("\t")[0]) < len(j["steps"])]
        want = [lab for _, lab in j["f"].steps_ev if lab != "done"]
        total += 1
        agree += got == want
    print(f"  visits column against the timeline's session.step entries, same step sequence: {agree} of {total} Jobs")
    wf_disp = [d for d in disputes if d["sid"] in jobs]
    print(f"disputes {len(disputes)}; on Jobs with a rail history {len(wf_disp)}; on sessions with none (task marks on plain Sessions) {len(disputes)-len(wf_disp)}")
    print("dispute kinds:", dict(collections.Counter(d['kind'] for d in disputes)))
    disputed_sids = {d["sid"] for d in disputes}
    on165 = [d for d in disputes if d["sid"] in rail_ids]
    print(f"disputes on the {len(rail_ids)} rail Jobs with visits: {len(on165)}, across {len({d['sid'] for d in on165})} Jobs; all {len(disputes)} are across {len(disputed_sids)} sessions")

    results = []
    for d in disputes:
        j = jobs.get(d["sid"])
        r = dict(d)
        r["resolved"] = bool(j and j["steps"])
        if r["resolved"]:
            f, steps = j["f"], j["steps"]
            r["O"] = old_place(f, steps, d["t"])
            for v in "SPM":
                r[v] = gate_place(f, steps, d["t"], v)
            r["named"] = next((i for i, s in enumerate(steps) if s["label"] == d["label"]), None)
            r["T"] = truth_role(d["kind"], d["why"])
            r["roles"] = {k: role_of(steps, r[k]) for k in "OSPM"}
            r["T"] = true_roles(steps, r["T"])
        results.append(r)
    JOBS.update(jobs)
    return jobs, disputes, results, disputed_sids


def tally(results):
    print("\n== Disputes: what each variant does")
    print("Columns per variant: not sent = gate-only sends no such notice at that moment (a non-gate fact made it);")
    print("same notice = gate-only holds the Job on the very step the notice named; then 'agrees' means the agent's reason puts the Job there.")
    print("Placement = the step shown at the notice's time. right/wrong are against the agent's reason (automatic judge); n/a = reason names no step.\n")
    kinds = ["on-step", "push-open", "edit-before-plan", "test-before-walk", "turn-end", "tasks"]
    rows = [r for r in results]
    for v in "SPM":
        print(f"-- variant {v}: {dict(S='strict', P='strict + own push', M='strict + own push + a later gate passes the steps before it')[v]}")
        print(f"{'kind':18}{'n':>4}{'not sent':>10}{'same notice':>13}{'  of which agrees':>18}{'  disagrees':>12}{'  n/a':>6}  |{'placement right':>16}{'wrong':>7}{'n/a':>6}{'old right':>11}{'old wrong':>11}")
        tot = collections.Counter()
        for k in kinds + ["ALL"]:
            sub = [r for r in rows if k == "ALL" or r["kind"] == k]
            c = collections.Counter()
            c["n"] = len(sub)
            for r in sub:
                if not r["resolved"]:
                    c["unres"] += 1
                    # a task mark on a plain Session: not sent either way
                    if r["kind"] == "tasks":
                        c["notsent"] += 1
                    continue
                if r["kind"] == "on-step" and r[v] == r["named"]:
                    c["same"] += 1
                    ag = r["T"] is not None and fits(r["T"], r["roles"][v])
                    c["agree" if ag else ("na" if r["T"] is None else "disagree")] += 1
                else:
                    c["notsent"] += 1
                if r["T"] is None:
                    c["pna"] += 1
                elif fits(r["T"], r["roles"][v]):
                    c["pright"] += 1
                else:
                    c["pwrong"] += 1
                if r["T"] is not None:
                    c["oright" if fits(r["T"], r["roles"]["O"]) else "owrong"] += 1
            print(f"{k:18}{c['n']:>4}{c['notsent']:>10}{c['same']:>13}{c['agree']:>18}{c['disagree']:>12}{c['na']:>6}  |{c['pright']:>16}{c['pwrong']:>7}{c['pna']:>6}{c['oright']:>11}{c['owrong']:>11}")
        print()


def outcome(r, v, T):
    """A: gone (no notice, or none that is wrong), B: gone but the step shown is wrong, C: sent and still disputed, D: sent and right."""
    if r["kind"] == "tasks":
        return "A"
    if not r["resolved"]:
        return "U"
    sent = r["kind"] == "on-step" and r[v] == r["named"]
    right = T is not None and fits(T, r["roles"][v])
    if sent:
        return "D" if right else "C"
    if T is None or right:
        return "A"
    return "B"


def outcomes(results):
    print("\n== Outcome of each of the 205 disputes (automatic judge)")
    print("A gone: no notice, and the step shown is right or the reason names none (all 50 task marks are here)")
    print("B gone, wrong step shown: no notice, but gate-only's step is not where the reason puts the Job")
    print("C sent and still disputed: the notice is sent on the same step and the reason says otherwise (or disputes something else)")
    print("D sent and right: the notice is sent on the same step and the reason agrees")
    print("U the Job's workflow could not be rebuilt (2 Jobs that changed workflow part way)")
    print(f"{'variant':10}{'A':>6}{'B':>6}{'C':>6}{'D':>6}{'U':>4}{'  disputes avoided (A+D)':>26}{'  of the 153 step disputes: A+D':>32}")
    for v in "SPM":
        c = collections.Counter(outcome(r, v, r.get("T")) for r in results)
        step = collections.Counter(outcome(r, v, r.get("T")) for r in results if r["kind"] != "tasks")
        print(f"{v:10}{c['A']:>6}{c['B']:>6}{c['C']:>6}{c['D']:>6}{c['U']:>4}{c['A']+c['D']:>26}{step['A']+step['D']:>32}")
    print("\nBy kind of notice (A / B / C / D / U):")
    for v in "SPM":
        for k in ["on-step", "push-open", "edit-before-plan", "test-before-walk", "turn-end", "tasks"]:
            c = collections.Counter(outcome(r, v, r.get("T")) for r in results if r["kind"] == k)
            print(f"  {v} {k:18}{sum(c.values()):>3}   A {c['A']:>2}  B {c['B']:>2}  C {c['C']:>2}  D {c['D']:>2}  U {c['U']:>2}")
    base = collections.Counter(("right" if r.get("T") and fits(r["T"], r["roles"]["O"]) else "wrong" if r.get("T") else "n/a")
                               for r in results if r["resolved"] and r["kind"] != "tasks")
    print(f"old rail, same judge, the step disputes it could place: {dict(base)}")
    placed = [r for r in results if r["resolved"] and r.get("T")]
    print(f"\nOf the {len(placed)} disputes whose reason names a step, with the same judge:")
    print(f"  old rail right on {sum(fits(r['T'], r['roles']['O']) for r in placed)}")
    for v in "SPM":
        right = [r for r in placed if fits(r["T"], r["roles"][v])]
        both = sum(1 for r in right if fits(r["T"], r["roles"]["O"]))
        lost = sum(1 for r in placed if fits(r["T"], r["roles"]["O"]) and not fits(r["T"], r["roles"][v]))
        late = 0
        for r in placed:
            if not fits(r["T"], r["roles"][v]):
                j = JOBS[r["sid"]]
                later = gate_place(j["f"], j["steps"], add_seconds(r["t"], 600), v)
                late += fits(r["T"], role_of(j["steps"], later))
        print(f"  {v} right on {len(right)} ({len(right)/len(placed):.0%}); of the old rail's right ones, gate-only gets {lost} wrong; of its wrong ones, {late} are right ten minutes later (the gate closed just after the notice)")


# Hand labels: the 30 drawn with random.Random(41) from the notice ids in sorted order, read against the agent's
# reason by the author of this spike. The role is where the reason says the Job really was; None where the reason
# disputes something that is no step (a task mark, an owner's stop).
HAND = {
    "4ffc5c8a-20261025T195927306Z-803": ("done", "merged #2874"),
    "04cc27f6-20261031T001450043Z-237": (None, "owner stopped the Job; the dispute is about what the notice asks"),
    "ec9b00ee-20261031T160843089Z-892": ("walk", "plan approved; first group is a mock he must walk before wiring"),
    "98632ca8-20261030T024154736Z-184": ("done", "no walk expected, work merged #3040"),
    "803c9e4c-20261023T120744628Z-784": ("done", "merged #2795"),
    "4e9dba06-20261105T011401993Z-35": (None, "task marks, parallel subagents"),
    "63c06822-20261103T172337260Z-15": ("build", "walk approved earlier; tests written; a preview check page was shown after"),
    "fac57741-20261030T012622306Z-231": (None, "task marks, parallel agents"),
    "fc9889b4-20261102T174030060Z-442": (None, "task marks credited to the wrong group"),
    "3af57ce3-20261103T003539464Z-25": ("build", "past Discuss: he approved the walk, work is in Refine"),
    "37bb03ba-20261030T015637306Z-78": ("walk", "no walk shown or approved; the push was the mock subagent's"),
    "200c15cc-20261103T202323556Z-307": (None, "task marks, parallel subagents"),
    "d9b709cf-20261103T150913980Z-87": ("build", "Refine unfinished; a wiring agent committed part of it"),
    "2d9549b2-20261103T170331982Z-104": (None, "task marks, one agent"),
    "ec9b00ee-20261101T021112072Z-113": ("build", "Refine not done, tasks 14 to 17 to build"),
    "a8e305c0-20261030T133106586Z-605": ("build", "walk and plan v2 approved before any test"),
    "3af57ce3-20261103T003623104Z-36": ("build", "walk approved, tests are the next work"),
    "8e6b8869-20261028T044034927Z-231": ("done", "merged #2967, no walk expected"),
    "e3e67114-20261022T204525479Z-397": ("plan", "no repository file edited"),
    "4e9dba06-20261104T064405250Z-706": (None, "task marks, parallel subagents"),
    "628d0030-20261102T024052467Z-1054": ("build", "plan v1 approved, push is the build's first commits"),
    "a9161237-20261029T192728115Z-339": (None, "task marks, overlapping group paths"),
    "520be125-20261023T024755958Z-342": ("plan", "plan v2 recorded and waiting; the edits were under approved v1"),
    "a3e259f0-20261102T130651919Z-1170": ("done", "pushed and merged #3162"),
    "d8026526-20261027T184004347Z-391": ("done", "plan's tasks complete, #2934 merged"),
    "b9e2d30a-20261028T143222941Z-0": ("tests", "walk already approved, the step should advance to Tests"),
    "63c06822-20261103T141700560Z-1172": (None, "task marks, one subagent"),
    "f5370cc9-20261027T131632259Z-173": ("done", "merged #2925"),
    "6e420b67-20261101T152420353Z-544": ("done", "merged #3137, slot released"),
    "63c06822-20261103T154028842Z-320": (None, "task marks, two engineers"),
}


HAND2 = {
    "181d56a9-20261031T000743615Z-67": ("build", "v1 pushed under an approved plan; v2 recorded after, so the Job is on its Fix again"),
    "38dfdbe1-20261103T173443209Z-127": ("deliver", "branch pushed, PR open with auto-merge"),
    "e4d92237-20261105T032107926Z-551": (None, "task marks, a background subagent"),
    "d11a358f-20261101T152452709Z-569": ("deliver", "pushed, PR open"),
    "33b2b07b-20261029T184758015Z-156": (None, "task marks, edits outside every group"),
    "f1bf1d8e-20261022T040058677Z-1266": ("build", "13 tasks open, the push was a merge of main"),
    "63c06822-20261103T183452059Z-71": ("done", "merged #3223"),
    "735fb544-20261102T174509074Z-37": (None, "task marks, PR in the merge queue"),
    "4d538bb2-20261024T014144163Z-82": ("done", "merged #2843, finish_up run"),
    "37fc8910-20261023T203203069Z-38": ("done", "merged #2827"),
    "fac57741-20261030T034610831Z-697": (None, "task marks, agent just dispatched"),
    "90a4de69-20261030T134106716Z-759": ("tests", "walk approved, fix committed, next is Tests"),
    "e37449ac-20261023T014026751Z-594": ("done", "design merged #2772"),
    "887f02f4-20261027T140136812Z-821": ("plan", "the push was the mock walk, before the plan was recorded"),
    "08c5fd44-20261031T161011948Z-910": ("plan", "only the Draft step's own document edited"),
    "ec9b00ee-20261101T002809534Z-2183": (None, "task marks, task 21 against task 10"),
    "200c15cc-20261103T205023858Z-182": (None, "task marks, mock before wiring"),
    "e2465dc1-20261023T180328018Z-159": ("build", "29 tasks open, two agents running"),
    "2d9549b2-20261103T213655917Z-682": ("deliver", "every task finished, Refine committed"),
    "42d07158-20261026T013119805Z-774": ("done", "merged #2875"),
    "37bb03ba-20261030T032248081Z-397": ("done", "merged #3042"),
    "5e55e0fd-20261026T023135608Z-309": ("done", "merged #2878"),
    "c443558f-20261030T142820695Z-1436": ("tests", "fix committed and walk approved; Tests, not Fix"),
    "c2d23c4c-20261023T144353637Z-1420": ("build", "Implement unfinished, walk unapproved"),
    "4d27072d-20261025T155358206Z-222": ("build", "plan v1 approved and building; v2 records walk notes"),
    "5bbc95ca-20261104T145454224Z-80": (None, "task marks, parallel agents"),
    "4b2744bd-20261029T133702787Z-369": (None, "task marks, a finished task marked started"),
    "c2d23c4c-20261023T114046553Z-533": ("build", "first of seven groups done"),
    "11632803-20261028T143108396Z-881": ("done", "merged #2971"),
    "9300002d-20261027T131904980Z-215": ("build", "plan v1 approved; the edits are Implement"),
}


def hand_labels(results, jobs, hand, seed, exclude, title):
    rows = sorted((r for r in results if r["row"] not in exclude), key=lambda r: r["row"])
    random.Random(41).shuffle(rows) if seed == 41 else random.Random(seed).shuffle(rows)
    sample = rows[:30]
    assert {r["row"] for r in sample} == set(hand), "the sample drifted from the labels"
    nm = lambda r, i: "DONE" if i == len(jobs[r["sid"]]["steps"]) else jobs[r["sid"]]["steps"][i]["label"]
    print(f"\n== {title}")
    print(f"{'#':>2} {'kind':17}{'hand':>8}{'auto':>8}  {'old':14}{'S':14}{'P':14}{'M':14} reading")
    agree = 0
    for n, r in enumerate(sample):
        h, note = hand[r["row"]]
        agree += h == r.get("T")
        print(f"{n:>2} {r['kind']:17}{str(h):>8}{str(r.get('T')):>8}  {nm(r, r['O']):14.13}{nm(r, r['S']):14.13}{nm(r, r['P']):14.13}{nm(r, r['M']):14.13} {note}")
    if seed == 41:
        v1 = sum(hand[r["row"]][0] == truth_role_v1(r["kind"], r["why"]) for r in sample)
        print(f"\nthe first version of the judge, run before it was changed, matched {v1} of 30; the second matches this sample on {agree}, which is no longer an independent test")
    else:
        print(f"\nthe second version of the judge, frozen before this sample was drawn, matches the hand label on {agree} of 30")
    step = [r for r in sample if r["kind"] != "tasks"]
    print(f"tasks: {sum(1 for r in sample if r['kind']=='tasks' and r.get('T') is None)} of {sum(1 for r in sample if r['kind']=='tasks')} agree; of the {len(step)} step disputes, hand says a step in {sum(1 for r in step if hand[r['row']][0])}, auto in {sum(1 for r in step if r.get('T'))}")
    print("Outcome classes on the 30, hand label against automatic label:")
    for v in "SPM":
        h = collections.Counter(outcome(r, v, hand[r["row"]][0]) for r in sample)
        a = collections.Counter(outcome(r, v, r.get("T")) for r in sample)
        same = sum(outcome(r, v, hand[r["row"]][0]) == outcome(r, v, r.get("T")) for r in sample)
        print(f"  {v}: hand A{h['A']} B{h['B']} C{h['C']} D{h['D']}   auto A{a['A']} B{a['B']} C{a['C']} D{a['D']}   same class on {same} of 30")
    placed = [r for r in sample if hand[r["row"]][0]]
    print(f"Of the {len(placed)} hand-labelled disputes that name a step, the step shown at the notice's time is right for: old rail " + str(
        sum(fits(hand[r["row"]][0], r["roles"]["O"]) for r in placed)) + ", " + ", ".join(
        f"{v} {sum(fits(hand[r['row']][0], r['roles'][v]) for r in placed)}" for v in "SPM"))


def walk_holds(jobs, disputed):
    print("\n== Why variant M holds Jobs on a walk step at their end (control Jobs, rail Jobs with visits)")
    rail_ids = {r[0] for r in con.execute("select session_id from session_rail where visits<>''")}
    c = collections.Counter()
    for s, j in jobs.items():
        if s in disputed or s not in rail_ids or not j["steps"]:
            continue
        st, f = j["steps"], j["f"]
        o, m = old_place(f, st, "9999"), gate_place(f, st, "9999", "M")
        if m < o and st[m]["walks"]:
            shown = len(f.windows)
            c["no window ever shown (walk passed over: nothing to see)" if shown == 0 else
              "windows shown, none approved" if not f.walk_ok else "windows shown, some approved, a later or other one not"] += 1
    for k, v in c.items():
        print(f"  {v:3}  {k}")


END_FACTS = [
    ("plan_approved", "rail_ends.rs:15", "session_plan.approved_at (newest version only); the thread line 'The owner approved plan vN' keeps the earlier ones", "yes"),
    ("walk_approved_all / walk_approved", "rail_ends.rs:16,19", "a You message 'Approved <address>' (walking.rs:22) beside the window row", "yes; dismissed and replaced windows are read from other tables"),
    ("sketch_approved", "rail_ends.rs:20", "a You message 'Approved sketch: ' (sketching.rs:203), no structured row", "yes, by text prefix"),
    ("draft_approved", "rail_ends.rs:21", "draft row settled.how approved or fixed", "yes"),
    ("idea_picked", "rail_ends.rs:22", "ask row options with a preview, and the ':said' answer message", "weak: parsed from answer text"),
    ("document_approved", "rail_ends.rs:23", "ledger_attachments artifact detail settled='approved', settled_at", "yes"),
    ("merged", "rail_ends.rs:14", "timeline pr.merged, ledger pr detail state=merged", "yes"),
    ("pr_opened", "rail_ends.rs:30", "timeline pr.opened", "yes"),
    ("checked", "rail_ends.rs:28", "session_check_runs, timeline pr.checks", "a Check run yes; 'green' is a rule over the latest run of each Check"),
    ("pushed", "rail_ends.rs:25", "timeline session.pushed, branch in detail", "partly: no branch on {nobranch} of {pushes} pushes, so own against a subagent's cannot be told"),
    ("committed", "rail_ends.rs:29", "timeline session.committed", "no: {commits} rows in the whole store"),
    ("edited", "rail_ends.rs:27", "timeline session.edited", "seen, not a gate"),
    ("walk_shown", "rail_ends.rs:26", "timeline session.walked, window rows", "seen, the Job's act, not a gate"),
    ("plan_recorded", "rail_ends.rs:24", "session_plan.recorded_at", "seen, the Job's act, not a gate"),
    ("replied", "rail_ends.rs:17", "You messages", "seen, but a reply is not an approval"),
    ("lifecycle (job: landed)", "rail_ends.rs:33", "lifecycle_instances, lifecycle_moves", "seen by Fleet; not in the brief's gate list, held open in this replay"),
    ("tasks completed", "rail.rs:355-358 (passed: follows_plan)", "session_plan_tasks listed/marked", "no: the agent's TaskUpdate or Fleet's mark from edits, which the 50 task disputes name"),
]


def gates(jobs):
    nob = con.execute("select sum(detail is null), count(*) from timeline_events where kind='session.pushed'").fetchone()
    com = con.execute("select count(*) from timeline_events where kind='session.committed'").fetchone()[0]
    print("\n== Gates the shipped workflows need, and what backs them in the store")
    print(f"{'end':36}{'defined':20}{'store fact':80}seen as a gate")
    for kind, line, fact, seen in END_FACTS:
        print(f"{kind:36}{line:20}{fact:80}{seen.format(nobranch=nob[0], pushes=nob[1], commits=com)}")
    print("\nSteps of the workflows as shipped today, and what ends each (from ends_of; * = no gate under the brief's list):")
    for name in sorted(VERSIONS):
        steps = VERSIONS[name][-1][1]
        for i, st in enumerate(steps):
            e = ends_of(steps, i)
            mark = "*" if not any(x in STRICT_GATES for x in e) else " "
            print(f"  {name:12}{i} {st['label']:18}{mark} ends on: {', '.join(e) or 'nothing'}")


def control(jobs, disputed_sids):
    print("\n== Control: Jobs with no rail dispute, old rail's end state against gate-only's")
    print("A Job is 'held back' when gate-only's step at the Job's last moment is an earlier step than the old rail's; 'stuck at' names the first step whose end is no gate.")
    groups = {"no dispute": [s for s in jobs if s not in disputed_sids], "disputed": [s for s in jobs if s in disputed_sids]}
    rail_ids = {r[0] for r in con.execute("select session_id from session_rail where visits<>''")}
    for gname, sids in groups.items():
        for pop, pred in (("rail Jobs with visits (the brief's 165)", lambda s: s in rail_ids), ("all Jobs with a rail history", lambda s: True)):
            use = [s for s in sids if pred(s) and jobs[s]["steps"]]
            print(f"\n{gname}, {pop}: {len(use)} Jobs")
            for v in "SPM":
                held = same = ahead = 0
                stuck = collections.Counter()
                landed_held = landed = 0
                for s in use:
                    j = jobs[s]
                    steps, f = j["steps"], j["f"]
                    t = "9999"
                    o = old_place(f, steps, t)
                    g = gate_place(f, steps, t, v)
                    if g < o:
                        held += 1
                        stuck[(ends_of(steps, g)[0] if ends_of(steps, g) else "no end")] += 1
                    elif g == o:
                        same += 1
                    else:
                        ahead += 1
                    if f.pr_merge:
                        landed += 1
                        if g < len(steps):
                            landed_held += 1
                print(f"  {v}: same {same}, held back (earlier step) {held}, ahead {ahead}; of {landed} Jobs whose PR merged, {landed_held} still show a step at the end; held back on end kind {dict(stuck)}")
    # time share
    print("\nTime share held back (control Jobs, rail Jobs with visits): fraction of the Job's life where gate-only is on an earlier step than the old rail")
    use = [s for s in groups["no dispute"] if s in rail_ids and jobs[s]["steps"]]
    for v in "SPM":
        tot = held = 0.0
        for s in use:
            j = jobs[s]
            f, steps = j["f"], j["steps"]
            marks = sorted({a for a, _ in f.steps_ev} | {j["ended"]})
            for a, b in zip(marks, marks[1:]):
                dt = (parse(b) - parse(a))
                if dt <= 0 or dt > 6 * 3600:
                    continue
                tot += dt
                if gate_place(f, steps, a, v) < old_place(f, steps, a):
                    held += dt
        print(f"  {v}: {held/tot:.0%} of {tot/3600:.0f} Job-hours (gaps over 6 hours left out)")


def add_seconds(ts, n):
    return datetime.datetime.fromtimestamp(parse(ts) + n, datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.000Z")


def parse(ts):
    import datetime
    return datetime.datetime.strptime(ts[:19], "%Y-%m-%dT%H:%M:%S").replace(tzinfo=datetime.timezone.utc).timestamp()


if __name__ == "__main__":
    jobs, disputes, results, disputed = main()
    tally(results)
    outcomes(results)
    hand_labels(results, jobs, HAND, 41, set(), "The first 30 hand-labelled disputes")
    hand_labels(results, jobs, HAND2, 42, set(HAND), "A second 30, drawn after the judge was frozen")
    control(jobs, disputed)
    walk_holds(jobs, disputed)
    gates(jobs)
