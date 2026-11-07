#!/usr/bin/env python3
"""Reads one run of spike 46 and prints the facts the score table needs. `python3 -I score.py RUN_DIR`.

  refusals        every tool call answered with "Gate open: ...", with the time, the tool, its input and the gate record then
  plan            the groups of each record_plan call (titles, task count) against the three required titles
  task claims     every TaskCreate / TaskUpdate with the group the plan says it belongs to, when known
  gates           the gate record, with closed_at relative to the first event
  after-gate      the agent's first tool call of each kind after each gate closed

Stdlib only. Reads the transcript copy in RUN_DIR/transcripts, gates.jsonl and record.json.
"""
import glob
import json
import sys
from datetime import datetime, timezone

REQUIRED = ["Find the cause and plan the fix", "Fix and test", "Deliver"]


def ts(text):
    return datetime.fromisoformat(text.replace("Z", "+00:00")).astimezone(timezone.utc).timestamp()


def main():
    run = sys.argv[1]
    events = []  # (epoch, kind, payload)
    uses = {}
    for path in sorted(glob.glob(f"{run}/transcripts/*.jsonl")):
        if "subagents" in path:
            continue
        for line in open(path):
            try:
                d = json.loads(line)
            except ValueError:
                continue
            at, msg = d.get("timestamp"), d.get("message")
            if not at or not isinstance(msg, dict) or not isinstance(msg.get("content"), list):
                continue
            for b in msg["content"]:
                if b.get("type") == "tool_use" and b["id"] in uses:
                    continue  # the transcript is kept twice, under the clone and under the slot
                if b.get("type") == "tool_use":
                    uses[b["id"]] = (ts(at), b["name"], b.get("input") or {})
                    events.append((ts(at), "use", (b["id"], b["name"], b.get("input") or {})))
                if b.get("type") == "tool_result":
                    c = b.get("content")
                    text = " ".join(x.get("text", "") for x in c) if isinstance(c, list) else str(c)
                    events.append((ts(at), "result", (b.get("tool_use_id"), text, bool(b.get("is_error")))))
    events.sort(key=lambda e: e[0])
    t0 = events[0][0]
    for line in open(f"{run}/gates.jsonl"):
        d = json.loads(line)
        if d["event"] == "keeper_started":
            t0 = ts(d["at"])  # the timeline's zero: the keeper starts before the Job is told anything
    rel = lambda t: "+%02d:%02d" % (max(t - t0, 0) // 60, max(t - t0, 0) % 60)

    record = json.load(open(f"{run}/record.json"))["gates"]
    closed = {k: (ts(g["closed_at"]) if g["closed_at"] else None) for k, g in record.items()}

    print("== gates")
    for k, t in closed.items():
        print(f"  {k}: {rel(t) if t else 'never closed'}  {record[k]['fact']}")

    print("== refusals (Gate open)")
    n = 0
    for at, kind, p in events:
        if kind == "result" and "Gate open:" in p[1] and p[2]:
            n += 1
            _, name, inp = uses.get(p[0], (0, "?", {}))
            state = ",".join(k for k, t in closed.items() if t and t <= at) or "none closed"
            what = inp.get("command") or inp.get("file_path") or json.dumps(inp)[:80]
            print(f"  {rel(at)} {name} {str(what)[:90]!r} -> {p[1].strip()[:110]!r} (closed by then: {state})")
    print(f"  count {n}")

    print("== plan")
    for at, kind, p in events:
        if kind == "use" and p[1].endswith("record_plan"):
            groups = (p[2].get("body") or p[2]).get("groups", [])
            titles = [g.get("title") for g in groups]
            print(f"  {rel(at)} record_plan: {len(groups)} groups; titles {titles}; tasks {[len(g.get('tasks', [])) for g in groups]}; matches template: {titles == REQUIRED}")

    print("== task calls")
    for at, kind, p in events:
        if kind == "use" and p[1] in ("TaskCreate", "TaskUpdate", "TodoWrite"):
            print(f"  {rel(at)} {p[1]} {json.dumps(p[2])[:150]}")

    print("== first call of each kind after each gate closed")
    for k, t in closed.items():
        if not t:
            continue
        seen = {}
        for at, kind, p in events:
            if kind == "use" and at > t and p[1] in ("Edit", "Write", "Bash", "mcp__armada-fleet__ask_the_person", "mcp__armada-fleet__all_done", "mcp__armada-fleet__waiting_for"):
                seen.setdefault(p[1], (at, str(p[2].get("command") or p[2].get("file_path") or "")[:70]))
        print(f"  after {k} ({rel(t)}): " + "; ".join(f"{n} {rel(a)} {w!r}" for n, (a, w) in seen.items()))


if __name__ == "__main__":
    main()
