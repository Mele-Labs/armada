#!/usr/bin/env python3
"""What a stretch's transcript shows, for spike 45.

  python3 -I transcripts.py handoff --home HOME --session ID      print the last text block that starts with HANDOFF
  python3 -I transcripts.py stats   --home HOME --session ID      JSON: tool calls by name, the files read, the end time
"""
import argparse
import collections
import glob
import json
import re


def events(home, sid):
    seen = set()
    for path in sorted(glob.glob(f"{home}/.claude/projects/**/*.jsonl", recursive=True)):
        if sid not in path or "subagents" in path:
            continue
        for line in open(path):
            try:
                d = json.loads(line)
            except ValueError:
                continue
            u = d.get("uuid")
            if u and u in seen:
                continue
            seen.add(u)
            yield d


def rel(p):
    return re.sub(r"^.*?/repo/(\.armada/slots/slot-\d+/)?", "", p)


PATHY = re.compile(r"[\w@./-]+\.(?:tsx?|json|md|ya?ml|rs|toml|sh|py)\b")


def stats(home, sid):
    calls, reads, bash_reads = collections.Counter(), collections.Counter(), collections.Counter()
    first = last = None
    turns = 0
    for d in events(home, sid):
        at = d.get("timestamp")
        if at:
            first = first or at
            last = at
        msg = d.get("message")
        if d.get("type") != "assistant" or not isinstance(msg, dict) or not isinstance(msg.get("content"), list):
            continue
        for b in msg["content"]:
            if b.get("type") != "tool_use":
                continue
            name, inp = b["name"], b.get("input") or {}
            calls[name] += 1
            if name == "Read" and inp.get("file_path"):
                reads[rel(inp["file_path"])] += 1
            elif name in ("Grep", "Glob"):
                pass
            elif name == "Bash":
                c = inp.get("command", "")
                if re.match(r"\s*(cd [^&;]+(&&|;)\s*)?(cat|head|tail|sed|rg|grep|wc|less|nl|awk)\b", c):
                    for p in PATHY.findall(c):
                        bash_reads[rel(p)] += 1
    return dict(calls=dict(calls), total=sum(calls.values()), reads=dict(reads), bash_reads=dict(bash_reads), first=first, last=last)


def handoff(home, sid):
    best = None
    for d in events(home, sid):
        msg = d.get("message")
        if d.get("type") == "assistant" and isinstance(msg, dict) and isinstance(msg.get("content"), list):
            for b in msg["content"]:
                m = b.get("type") == "text" and re.search(r"(?m)^\s*(#+\s*)?\**HANDOFF\b", b["text"])
                if m:
                    best = b["text"][m.start():].strip()
    return best


ap = argparse.ArgumentParser()
ap.add_argument("cmd", choices=["handoff", "stats"])
ap.add_argument("--home", required=True)
ap.add_argument("--session", required=True)
a = ap.parse_args()
if a.cmd == "handoff":
    h = handoff(a.home, a.session)
    print(h if h else "", end="")
else:
    print(json.dumps(stats(a.home, a.session)))
