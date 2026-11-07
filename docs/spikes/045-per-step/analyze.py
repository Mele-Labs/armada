#!/usr/bin/env python3
"""Score one per-step run: wall time, tool calls, tokens and an estimated cost per stretch,
and the files a later stretch read that an earlier one had read.

  python3 -I analyze.py --scratch DIR --run RUNDIR [--baseline-session ID]

Cost is an estimate from token counts at list prices per million tokens (input 3, cache write 3.75,
cache read 0.30, output 15), the same formula for every row, so rows compare with each other.
"""
import argparse
import glob
import json
import os
import re
import sqlite3
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
PRICE = dict(i=3.0, cw=3.75, cr=0.30, o=15.0)


def usage(db, sid):
    con = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    r = con.execute(
        "select count(*),sum(input_tokens),sum(cache_creation_tokens),sum(cache_read_tokens),sum(output_tokens) from session_turn_usage where session_id=?",
        (sid,),
    ).fetchone()
    s = con.execute("select started_at, ended_at from sessions where id=?", (sid,)).fetchone()
    turns, i, cw, cr, o = [x or 0 for x in r]
    cost = (i * PRICE["i"] + cw * PRICE["cw"] + cr * PRICE["cr"] + o * PRICE["o"]) / 1e6
    return dict(turns=turns, input=i, cache_write=cw, cache_read=cr, output=o, cost=cost, started=s[0], ended=s[1])


def minutes(a, b):
    from datetime import datetime

    f = lambda t: datetime.fromisoformat(t.replace("Z", "+00:00")).timestamp()
    return (f(b) - f(a)) / 60 if a and b else None


def stats(home, sid):
    out = subprocess.run(["python3", "-I", f"{HERE}/transcripts.py", "stats", "--home", home, "--session", sid], capture_output=True, text=True)
    return json.loads(out.stdout)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--scratch", required=True)
    ap.add_argument("--run", required=True)
    ap.add_argument("--baseline-session")
    a = ap.parse_args()
    home = f"{a.scratch}/home/user"
    db = f"{home}/Library/Application Support/Armada/armada.db"
    sids = sorted((int(re.match(r"session_id-(\d)", os.path.basename(p)).group(1)), os.path.basename(p).split("-", 2)[2]) for p in glob.glob(f"{a.run}/session_id-*"))
    rows, st = [], {}
    for n, sid in sids:
        u, s = usage(db, sid), stats(home, sid)
        st[n] = s
        rows.append((n, sid, u, s))
    print("| row | Job | wall min | tool calls | turns | input | cache write | cache read | output | est. cost $ |")
    print("|---|---|---|---|---|---|---|---|---|---|")
    tot = dict(wall=0, calls=0, cost=0.0)
    for n, sid, u, s in rows:
        w = minutes(u["started"], u["ended"])
        tot["wall"] += w or 0
        tot["calls"] += s["total"]
        tot["cost"] += u["cost"]
        print(f"| stretch {n} | {sid[:8]} | {w:.1f} | {s['total']} | {u['turns']} | {u['input']} | {u['cache_write']} | {u['cache_read']} | {u['output']} | {u['cost']:.3f} |")
    print(f"| all stretches | | {tot['wall']:.1f} | {tot['calls']} | | | | | | {tot['cost']:.3f} |")
    if a.baseline_session:
        u, s = usage(db, a.baseline_session), stats(home, a.baseline_session)
        print(f"| single-agent baseline | {a.baseline_session[:8]} | {minutes(u['started'], u['ended']):.1f} | {s['total']} | {u['turns']} | {u['input']} | {u['cache_write']} | {u['cache_read']} | {u['output']} | {u['cost']:.3f} |")
    print()
    print("Calls by name per stretch:")
    for n, sid, u, s in rows:
        print(f"- stretch {n}: " + ", ".join(f"{k} {v}" for k, v in sorted(s["calls"].items(), key=lambda kv: -kv[1])))
    print()
    print("Files read per stretch, and read again by a later one:")
    seen = {}
    for n, sid, u, s in rows:
        mine = set(s["reads"]) | set(s["bash_reads"])
        again = sorted(f for f in mine if any(f in seen[m] for m in seen))
        print(f"- stretch {n} read {len(mine)} files; {len(again)} of them an earlier stretch had read: {', '.join(again) if again else 'none'}")
        print(f"  all: {', '.join(sorted(mine)) if mine else 'none'}")
        seen[n] = mine
    print()
    ev = open(f"{a.run}/events.log").read().splitlines()
    print("Events:")
    for l in ev:
        print("  " + l.replace("\t", "  "))


main()
