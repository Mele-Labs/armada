#!/usr/bin/env python3
"""Watch every Rust compile on this Mac for the shared-builds spikes.

Every INTERVAL seconds it reads `ps` once. Each rustc it sees gets one record when it
exits: crate, check or build, its worktree, what started it, wall and CPU, and a hash of
every workspace crate's sources in that worktree at the moment it was first seen, so a
later pass can say whether two compiles in two slots built the same crate state.
Each cargo gets a record too, so runs that compiled nothing still count.

Usage: sampler.py OUT_DIR HOURS
"""
import hashlib, json, os, re, subprocess, sys, threading, time
from concurrent.futures import ThreadPoolExecutor

OUT, HOURS = sys.argv[1], float(sys.argv[2])
INTERVAL = 2.0
os.makedirs(OUT, exist_ok=True)
rustc_log = open(os.path.join(OUT, "rustc.jsonl"), "a", buffering=1)
cargo_log = open(os.path.join(OUT, "cargo.jsonl"), "a", buffering=1)
load_log = open(os.path.join(OUT, "load.jsonl"), "a", buffering=1)

def secs(s):
    # ps etime [[dd-]hh:]mm:ss and time [hh:]mm:ss.cc
    days = 0
    if "-" in s:
        d, s = s.split("-", 1); days = int(d)
    parts = [float(p) for p in s.split(":")]
    v = 0.0
    for p in parts: v = v * 60 + p
    return days * 86400 + v

def ps():
    out = subprocess.run(["ps", "-axww", "-o", "pid=,ppid=,etime=,time=,command="],
                         capture_output=True, text=True).stdout
    procs = {}
    for line in out.splitlines():
        f = line.split(None, 4)
        if len(f) < 5: continue
        try: procs[int(f[0])] = (int(f[1]), secs(f[2]), secs(f[3]), f[4])
        except ValueError: pass
    return procs

def cwd_of(pid):
    r = subprocess.run(["lsof", "-a", "-d", "cwd", "-p", str(pid), "-Fn"], capture_output=True, text=True)
    for l in r.stdout.splitlines():
        if l.startswith("n"): return l[1:]
    return None

def ancestry(pid, procs):
    chain, seen = [], set()
    while pid in procs and pid not in seen and len(chain) < 12:
        seen.add(pid)
        ppid, _, _, cmd = procs[pid]
        chain.append(cmd[:160])
        pid = ppid
    return chain

def worktree_of(path):
    p = path
    while p and p != "/":
        if os.path.exists(os.path.join(p, ".git")): return p
        p = os.path.dirname(p)
    return None

_fh = {}
def file_hash(path):
    try: st = os.stat(path)
    except OSError: return "gone"
    k = (path, st.st_size, st.st_mtime_ns)
    h = _fh.get(k)
    if h is None:
        with open(path, "rb") as f: h = hashlib.sha1(f.read()).hexdigest()
        _fh[k] = h
    return h

SKIP = {"target", "node_modules", ".git", ".playwright-mcp"}
def dir_hash(d):
    h = hashlib.sha1()
    for root, dirs, files in os.walk(d):
        dirs[:] = sorted(x for x in dirs if x not in SKIP)
        for name in sorted(files):
            p = os.path.join(root, name)
            h.update(os.path.relpath(p, d).encode()); h.update(file_hash(p).encode())
    return h.hexdigest()[:16]

def tree_state(wt, manifest_dir):
    """Hash of each crate dir under the workspace that holds manifest_dir, plus its lock."""
    state = {}
    base = manifest_dir
    for sub in ("crates", "xtask", "src"):
        d = os.path.join(base, sub)
        if not os.path.isdir(d): continue
        if sub == "crates":
            for c in sorted(os.listdir(d)):
                if os.path.isfile(os.path.join(d, c, "Cargo.toml")): state["crates/" + c] = dir_hash(os.path.join(d, c))
        else:
            state[sub] = dir_hash(d)
    for f in ("Cargo.lock", "Cargo.toml"):
        p = os.path.join(base, f)
        if os.path.exists(p): state[f] = file_hash(p)[:16]
    return state

def arg(args, name):
    for i, a in enumerate(args):
        if a == name and i + 1 < len(args): return args[i + 1]
        if a.startswith(name + "="): return a[len(name) + 1:]
    return None

HASHER = ThreadPoolExecutor(1)
_memo = {}
def state_of(wt):
    """The worktree's crate hashes, reused for two seconds: one build starts many rustc at once."""
    t, st = _memo.get(wt, (0, None))
    if time.time() - t > INTERVAL:
        try: st = tree_state(wt, wt)
        except Exception as e: st = {"error": str(e)}
        _memo[wt] = (time.time(), st)
    return st

live_rustc, live_cargo = {}, {}
end = time.time() + HOURS * 3600
last_load = 0
while time.time() < end:
    now = time.time()
    procs = ps()
    for pid, (ppid, et, cpu, cmd) in procs.items():
        exe = cmd.split(" ", 1)[0]
        base = os.path.basename(exe)
        if base in ("rustc", "clippy-driver") and "--crate-name" in cmd:
            r = live_rustc.get(pid)
            if r is None or r["cmd_head"] != cmd[:200]:
                args = cmd.split()
                cwd = cwd_of(pid) or ""
                src = next((a for a in args if a.endswith(".rs")), "")
                srcp = src if src.startswith("/") else os.path.join(cwd, src)
                wt = worktree_of(srcp) or worktree_of(cwd)
                in_registry = "/.cargo/registry/" in srcp or "/.cargo/git/" in srcp
                state = HASHER.submit(state_of, wt) if wt and not in_registry else None
                r = {
                    "pid": pid, "cmd_head": cmd[:200], "first": now, "last": now,
                    "crate": arg(args, "--crate-name"), "crate_type": arg(args, "--crate-type"),
                    "test": "--test" in args, "emit": arg(args, "--emit"),
                    "metadata": next((a.split("=", 1)[1] for a in args if a.startswith("metadata=")), None),
                    "opt": next((a for a in args if a.startswith("opt-level=")), None),
                    "debuginfo": next((a for a in args if a.startswith("debuginfo=")), None),
                    "src": srcp, "cwd": cwd, "worktree": wt, "workspace_member": bool(wt and not in_registry),
                    "out_dir": arg(args, "--out-dir"), "ancestry": ancestry(ppid, procs),
                    "state": state, "elapsed": et, "cpu": cpu,
                }
                live_rustc[pid] = r
            r["last"], r["elapsed"], r["cpu"] = now, et, cpu
        elif base in ("cargo", "cargo-nextest", "cargo-clippy") or (exe.endswith("/cargo") and len(cmd) > 6):
            c = live_cargo.get(pid)
            if c is None or c["cmd"] != cmd[:300]:
                c = {"pid": pid, "cmd": cmd[:300], "first": now, "cwd": cwd_of(pid),
                     "ancestry": ancestry(ppid, procs)}
                live_cargo[pid] = c
            c["last"], c["elapsed"] = now, et
    for table, log in ((live_rustc, rustc_log), (live_cargo, cargo_log)):
        for pid in [p for p, r in table.items() if p not in procs or (table is live_rustc and procs[p][3][:200] != r["cmd_head"])]:
            r = table.pop(pid); r["gone"] = now
            r.pop("cmd_head", None)
            if hasattr(r.get("state"), "result"): r["state"] = r["state"].result()
            log.write(json.dumps(r) + "\n")
    if now - last_load >= 30:
        la = subprocess.run(["sysctl", "-n", "vm.loadavg"], capture_output=True, text=True).stdout.split()
        load_log.write(json.dumps({"t": now, "load1": float(la[1]), "load5": float(la[2]), "load15": float(la[3]),
                                   "rustc_live": len(live_rustc), "cargo_live": len(live_cargo)}) + "\n")
        last_load = now
    time.sleep(max(0.0, INTERVAL - (time.time() - now)))
