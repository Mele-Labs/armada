#!/usr/bin/env python3
"""Copy of docs/spikes/046-refusals/timeline.py for spike 48: the keeper's events (keeper.jsonl) replace the gate log, subagent acts are marked sub,
and the person's acts carry their reason. Older header follows.
Copy of docs/spikes/044-seeded/timeline.py for spike 46 (the plan call is shown at length; no template, so no claims).
Spike 44: copy of docs/spikes/042-rig/timeline.py. Additions: source `gate` (gates.jsonl, the gate keeper), and
the agent's claimed step, shown in brackets on the agent's own acts.

One timeline for a run: the agent's transcript, the dev Fleet's store, the clone's git log
and the person's acts, merged in time order as one markdown table.

  python3 -I timeline.py --scratch DIR --session ID [--acts FILE] [--all-tools] [--out FILE]

Sources (the second column):
  agent    the agent's transcript JSONL under the scratch home: task tools, Edit/Write paths,
           Bash lines that commit, push, test or call armada, every Armada MCP call (record_plan,
           show_window, waiting_for, all_done ...), tool results that were refused, and the
           messages Fleet or the person sent it. --all-tools adds every other tool call.
  store    the dev Fleet's armada.db, opened read only: plan versions and approvals,
           session_plan_tasks marks, the agent's task list as Fleet saw it, notices and other
           rows of the thread, the Fleet timeline events, attachments the ledger changed.
  git      commits in the clone and its branches since the Job began.
  person   the acts person.py took.

Stdlib only; run it with `python3 -I`.
"""
import argparse
import collections
import glob
import json
import os
import re
import sqlite3
import subprocess
import sys

BASH_SHOWN = re.compile(r"\bgit (commit|push|merge|rebase|checkout|switch|branch|add|stash)\b|vitest|\barmada\b|\bpnpm\b|\bcargo\b|\bgh\b|\bbd\b")
GATE_KIND = re.compile(r"plan|step|walk|gate|workflow|sketch|document|draft|judge|check|pull|merge", re.I)


def stamp(text):
    """RFC 3339 to epoch milliseconds."""
    from datetime import datetime, timezone

    text = text.replace("Z", "+00:00")
    return int(datetime.fromisoformat(text).astimezone(timezone.utc).timestamp() * 1000)


def short(text, n=140):
    text = re.sub(r"\s+", " ", str(text)).strip()
    return text if len(text) <= n else text[: n - 1] + "…"


def agent_events(home, sid, all_tools):
    events, seen, counts = [], set(), collections.Counter()
    files = [f for f in glob.glob(f"{home}/.claude/projects/**/*.jsonl", recursive=True) if sid in f]
    for path in sorted(files):
        for line in open(path):
            try:
                d = json.loads(line)
            except ValueError:
                continue
            uuid = d.get("uuid")
            if uuid and uuid in seen:
                continue
            seen.add(uuid)
            at, msg = d.get("timestamp"), d.get("message")
            if not at or not isinstance(msg, dict):
                continue
            who = "sub" if "subagents" in path else "agent"
            content = msg.get("content")
            if d.get("type") == "assistant" and isinstance(content, list):
                for b in content:
                    if b.get("type") == "text" and b["text"].strip():
                        events.append((stamp(at), "agent", f"says: {short(b['text'], 110)}"))
                    if b.get("type") != "tool_use":
                        continue
                    name, inp = b["name"], b.get("input") or {}
                    counts[name] += 1
                    line_text = None
                    if name in ("TaskCreate", "TaskUpdate", "TaskList", "TaskGet", "TodoWrite"):
                        line_text = f"{name} {short(json.dumps(inp), 400 if name == 'TaskUpdate' else 160)}"
                    elif name in ("Edit", "Write", "NotebookEdit", "MultiEdit"):
                        line_text = f"{name} {inp.get('file_path') or inp.get('notebook_path')}"
                    elif name == "Bash":
                        command = inp.get("command", "")
                        if all_tools or BASH_SHOWN.search(command):
                            line_text = f"Bash {short(command, 150)}"
                    elif name.startswith("mcp__armada"):
                        line_text = f"{name.split('__', 2)[-1]} {short(json.dumps(inp), 900 if name.endswith('record_plan') else 170)}"
                    elif name in ("Agent", "Task"):
                        line_text = f"{name} {inp.get('subagent_type', '')} {short(inp.get('description', ''), 60)}"
                    elif all_tools:
                        line_text = f"{name} {short(json.dumps(inp), 100)}"
                    if line_text:
                        events.append((stamp(at), who, line_text))
            if d.get("type") == "user" and isinstance(content, list):
                for b in content:
                    if b.get("type") == "tool_result":
                        text = b.get("content")
                        text = " ".join(x.get("text", "") for x in text) if isinstance(text, list) else str(text)
                        if b.get("is_error") or re.search(r"hook error|has been denied|refused|blocked", text[:300], re.I):
                            events.append((stamp(at), who, f"tool refused or failed: {short(text, 170)}"))
                    elif b.get("type") == "text" and who == "agent":
                        events.append((stamp(at), "agent", f"message to agent: {short(b['text'], 150)}"))
            elif d.get("type") == "user" and isinstance(content, str) and who == "agent":
                events.append((stamp(at), "agent", f"message to agent: {short(content, 150)}"))
            if d.get("type") == "attachment":
                a = d.get("attachment", {})
                if a.get("type") in ("hook_blocking_error", "hook_non_blocking_error"):
                    events.append((stamp(at), "agent", f"hook error {a.get('hookName')}: {short(a.get('stderr') or a.get('content'), 150)}"))
                if a.get("type") == "hook_additional_context" and a.get("hookName") != "SessionStart":
                    events.append((stamp(at), "agent", f"hook context {a.get('hookName')}: {short(a.get('content'), 150)}"))
    return events, counts


def store_events(db, sid):
    con = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    con.row_factory = sqlite3.Row
    events, facts = [], collections.Counter()

    def add(at, text):
        if at:
            events.append((stamp(at), "store", text))

    for r in con.execute("select * from session_plan where session_id=?", (sid,)):
        add(r["recorded_at"], f"plan v{r['version']} recorded: {short(r['summary'], 100)}")
        add(r["approved_at"], f"plan v{r['version']} approved")
        facts["plan versions"] = r["version"]
        facts["plan approved"] += 1 if r["approved_at"] else 0
    for r in con.execute("select * from session_plan_changes where session_id=?", (sid,)):
        facts["plan changes"] += 1
    for r in con.execute("select * from session_plan_tasks where session_id=? order by position", (sid,)):
        label = f"{r['task_id']} {short(r['text'], 70)}"
        add(r["started_at"], f"plan task started: {label}")
        add(r["completed_at"], f"plan task completed: {label}")
        if r["marked"]:
            add(r["completed_at"] or r["touched_at"], f"plan task marked {r['marked']}: {label} ({short(r['marked_why'] or '', 60)})")
            facts["task marks"] += 1
        elif r["completed_at"] or r["started_at"]:
            facts["task marks"] += 1
    for r in con.execute("select * from session_build_tasks where session_id=? order by position", (sid,)):
        label = f"{r['task_id']} {short(r['text'], 70)}"
        add(r["first_seen_at"], f"agent task list: {label} ({r['status'] if not r['completed_at'] else 'seen'})")
        add(r["completed_at"], f"agent task list: completed {label}")
        facts["agent tasks seen"] += 1
    rows = collections.Counter()
    for r in con.execute("select body from session_rows where session_id=? order by seq", (sid,)):
        b = json.loads(r["body"])
        kind = b.get("kind")
        rows[kind] += 1
        if kind == "message":
            who = (b.get("from") or {}).get("kind")
            if b.get("notice"):
                add(b["at"], f"notice to agent ({b['notice']}): {short(b.get('text'), 140)}")
                facts["notices"] += 1
            elif who == "you":
                add(b["at"], f"person's message: {short(b.get('text'), 120)}")
            elif who == "session":
                add(b["at"], f"message from {(b.get('from') or {}).get('title', 'a session')}: {short(b.get('text'), 120)}")
        elif kind == "lease":
            add(b["at"], f"slot {b.get('slot')} leased on {b.get('branch')}")
        elif kind not in ("tool", "thinking"):
            add(b.get("at"), f"thread row {kind}: {short(json.dumps({k: v for k, v in b.items() if k not in ('id', 'at', 'kind')}), 120)}")
    facts["tool rows Fleet kept"] = rows.get("tool", 0)
    for r in con.execute("select * from timeline_events where subject_id=? order by seq", (sid,)):
        if r["kind"] in ("session.ran",):
            continue
        add(r["at"], f"event {r['kind']}: {short(r['says'], 110)}")
        if GATE_KIND.search(r["kind"]):
            facts["step and gate events"] += 1
    for r in con.execute(
        "select * from ledger_attachment_versions where holder_id=? and kind not in ('branch','slot') order by at", (sid,)
    ):
        add(r["at"], f"ledger {r['kind']} {short(r['target'], 60)} {r['state']} {short(r['detail'], 80) if r['detail'] != '{}' else ''}")
    for r in con.execute("select * from session_stops where session_id=?", (sid,)):
        add(r["at"], "stop recorded (Fleet's stop count)")
        facts["stops"] += 1
    for r in con.execute("select * from session_done where session_id=?", (sid,)):
        add(r["since"], f"all_done: {short(r['summary'] or '', 120)}")
    for r in con.execute("select * from session_waiting where session_id=?", (sid,)):
        add(r["since"], f"waiting (still open): {short(r['text'], 110)}")
    return events, facts


def git_events(repo, since_ms):
    events = []
    out = subprocess.run(
        ["git", "-C", repo, "log", "--branches", "--format=%ct%x09%h%x09%an%x09%D%x09%s", "-n", "200"],
        capture_output=True, text=True,
    ).stdout
    for line in out.splitlines():
        ct, h, _author, refs, subject = (line.split("\t") + ["", "", "", "", ""])[:5]
        if int(ct) * 1000 >= since_ms - 1000 and not subject.startswith("rig:"):
            events.append((int(ct) * 1000, "git", f"commit {h} {short(subject, 100)} {('[' + refs + ']') if refs else ''}".strip()))
    return events


def person_events(path):
    events = []
    if path and os.path.exists(path):
        for line in open(path):
            d = json.loads(line)
            events.append((stamp(d["at"]), "person", f"{d['act']} {d['item']} -> {d['status']} {short(d['text'], 70)} => {short(d['answer'], 70)} [{d.get('why', '')}]"))
    return events


def gate_events(run_dir):
    events = []
    path = f"{run_dir}/keeper.jsonl" if run_dir else None
    if path and os.path.exists(path):
        for line in open(path):
            d = json.loads(line)
            ev = d.pop("event")
            at = d.pop("at")
            if ev == "gate_closed":
                text = f"GATE CLOSED [{d['gate']}] {d['title']}: {d['fact']}"
            elif ev == "live_group":
                text = f"LIVE GROUP -> {d['pos']} {d['title']} [{d['gate']}] (plan v{d['plan_version']})"
            elif ev == "notice_sent":
                text = f"NOTICE to agent ({d['status']}): {d['text']}"
            elif ev == "checks_run":
                text = f"checks run at {d['sha']}: {'green' if d['green'] else 'RED'}, {d['said']}"
            elif ev == "plan_recorded":
                text = "plan recorded v%s: %s; template problems: %s" % (d["version"], " / ".join(f"{g['pos']} {g['title']} [{g['gate']}] x{g['tasks']}" for g in d["groups"]), d["template_problems"] or "none")
            elif ev in ("push_own_branch", "push_other_branch", "main_moved"):
                text = f"{ev.upper()} {d['ref']} {d['previous']}->{d['sha']} ({d['commits_past_main']} past main) live={d['live_group']}"
            else:
                text = f"{ev} {short(json.dumps(d), 200)}"
            events.append((stamp(at), "keeper", text))
    return events


def with_claims(events, tasks):
    """Adds `[claim: ...]` to the agent's own acts: the task its last TaskUpdate left in_progress."""
    label = {t["id"]: t["subject"].replace("Step: ", "S: ").replace("Gate: ", "G: ")[:28] for t in tasks}
    claim, out = None, []
    for at, source, text in events:
        if source == "agent" and text.startswith("TaskUpdate "):
            try:
                inp = json.loads(text[len("TaskUpdate "):])
                tid, status = str(inp.get("taskId")), inp.get("status")
                if status == "in_progress":
                    claim = tid
                elif status in ("completed", "deleted") and claim == tid:
                    claim = None
                text += f" => {label.get(tid, tid)} {status}"
            except ValueError:
                pass
        elif source == "agent" and re.match(r"(Edit|Write|Bash|record_plan|ask_the_person|waiting_for|all_done|show_)", text.split(" ")[0] if text else ""):
            text += f" [claim: {label.get(claim, 'none') if claim else 'none'}]"
        out.append((at, source, text))
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--scratch", required=True)
    ap.add_argument("--session", required=True)
    ap.add_argument("--acts")
    ap.add_argument("--run-dir")
    ap.add_argument("--all-tools", action="store_true")
    ap.add_argument("--out")
    args = ap.parse_args()
    home = f"{args.scratch}/home/user"
    db = f"{home}/Library/Application Support/Armada/armada.db"

    a_events, tool_counts = agent_events(home, args.session, args.all_tools)
    s_events, facts = store_events(db, args.session)
    started = min([e[0] for e in a_events + s_events] or [0])
    g_events = git_events(f"{args.scratch}/repo", started)
    p_events = person_events(args.acts)
    k_events = gate_events(args.run_dir)
    events = sorted(a_events + s_events + g_events + p_events + k_events, key=lambda e: (e[0], e[1]))
    if args.run_dir and os.path.exists(f"{args.run_dir}/template.json"):
        events = with_claims(events, json.load(open(f"{args.run_dir}/template.json"))["tasks"])
    if not events:
        sys.exit("no events found")
    t0 = events[0][0]
    from datetime import datetime, timezone

    lines = [
        f"Run of session {args.session}, first event {datetime.fromtimestamp(t0 / 1000, timezone.utc).strftime('%Y-%m-%d %H:%M:%S')} UTC. Times are minutes:seconds after it.",
        "",
        "| time | source | event |",
        "|---|---|---|",
    ]
    for at, source, text in events:
        s = (at - t0) // 1000
        lines.append(f"| +{s // 60:02d}:{s % 60:02d} | {source} | {text.replace('|', '/')} |")
    span = (events[-1][0] - t0) / 60000
    calls = sum(tool_counts.values())
    summary = (
        f"Summary: {span:.1f} minutes, {calls} tool calls by the agent ("
        + ", ".join(f"{k} {v}" for k, v in tool_counts.most_common(8))
        + f"). Fleet's store: {facts['tool rows Fleet kept']} tool rows, plan versions {facts['plan versions']} "
        f"(approved {facts['plan approved']}), plan task marks {facts['task marks']}, agent tasks seen {facts['agent tasks seen']}, "
        f"step and gate events {facts['step and gate events']}, notices to the agent {facts['notices']}, stops {facts['stops']}. "
        f"Person's acts {len(p_events)}, commits {len(g_events)}."
    )
    lines += ["", summary, ""]
    text = "\n".join(lines)
    if args.out:
        open(args.out, "w").write(text)
    print(text)


if __name__ == "__main__":
    main()
