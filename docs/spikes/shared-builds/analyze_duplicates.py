#!/usr/bin/env python3
"""Spike 1: how often two Rust compiles on this Mac built the same crate at the same state.

Reads sampler.py's rustc.jsonl and cargo.jsonl. Two compiles are the same unit when the
crate, its kind (lib, bin, test harness, build script), what it emitted (check or build),
its -C metadata hash, opt-level and debuginfo agree, and so do the sources of the crate and
of every workspace crate it depends on, with Cargo.lock. A third-party crate's sources are
fixed by its version, which its metadata hash already carries.

A schedule with one output store would run each unit once: every later compile of it in
another target dir is counted as a repeat. "At once" repeats overlapped in time with the
first, so even a schedule without a store would have had them waiting on one build.

Usage: analyze_duplicates.py DATA_DIR WORKSPACE_ROOT [EXCLUDE_PREFIX ...]
"""
import collections, hashlib, json, os, subprocess, sys

data, root = sys.argv[1], sys.argv[2]
exclude = sys.argv[3:]

def graph(manifest):
    m = json.loads(subprocess.run(["cargo", "metadata", "--format-version", "1", "--no-deps", "--offline",
                                   "--manifest-path", manifest], capture_output=True, text=True, check=True).stdout)
    by_name = {p["name"]: os.path.relpath(os.path.dirname(p["manifest_path"]), root) for p in m["packages"]}
    deps = {}
    for p in m["packages"]:
        d = by_name[p["name"]]
        deps[d] = [os.path.relpath(os.path.normpath(dep["path"]), root) for dep in p["dependencies"] if dep.get("path")]
    return deps

deps = graph(os.path.join(root, "Cargo.toml"))
deps.update(graph(os.path.join(root, "crates/bridge-native/Cargo.toml")))
deps["crates/bridge-native"] = deps.get("crates/bridge-native", ["crates/ipc", "crates/core-model"])

def closure(d):
    seen, todo = set(), [d]
    while todo:
        x = todo.pop()
        if x in seen: continue
        seen.add(x); todo.extend(deps.get(x, []))
    return sorted(seen)

def crate_dir(r):
    rel = os.path.relpath(r["src"], r["worktree"])
    parts = rel.split(os.sep)
    if parts[0] == "crates" and len(parts) > 1: return "crates/" + parts[1]
    return parts[0]

def who(r):
    chain = " | ".join(r.get("ancestry") or [])
    if "rust-analyzer" in chain: return "rust-analyzer"
    if "armada check" in chain: return "armada check (agent)"
    if "scripts/restart" in chain or "scripts/preview" in chain: return "restart or preview script"
    if "zsh -c" in chain or "bash -c" in chain: return "bare cargo (agent)"
    if "Armada Fleet.app" in chain or "armada serve" in chain: return "Fleet itself"
    return "sccache server (unattributed)"

def target_of(r):
    od = r.get("out_dir") or ""
    for marker in ("/debug/", "/release/"):
        if marker in od: return od.split(marker)[0]
    return od

rows = []
for l in open(os.path.join(data, "rustc.jsonl")):
    r = json.loads(l)
    if any((r.get("worktree") or r.get("out_dir") or "").startswith(p) for p in exclude): continue
    if any((r.get("out_dir") or "").startswith(p) for p in exclude): continue
    base = (r["crate"], r.get("crate_type"), r.get("test"), r.get("emit"), r.get("metadata"), r.get("opt"), r.get("debuginfo"))
    if r.get("workspace_member") and isinstance(r.get("state"), dict) and r.get("worktree"):
        st = r["state"]; d = crate_dir(r)
        if d not in st and d not in deps:
            key = state_key = None  # a crate outside the workspace graph, e.g. a scratch project; not matched
        else:
            h = hashlib.sha1()
            for x in closure(d): h.update((x + "=" + str(st.get(x))).encode())
            lock = "crates/bridge-native/Cargo.lock" if d == "crates/bridge-native" else "Cargo.lock"
            h.update(str(st.get(lock if lock in st else "Cargo.lock")).encode())
            key = base + (h.hexdigest(),)
            state_key = (r["crate"], r.get("crate_type"), d, h.hexdigest())
        kind = "workspace"
    elif not r.get("workspace_member"):
        key, kind = base, "third-party"
        state_key = None
    else:
        key, kind, state_key = None, "workspace", None
    rows.append({**r, "key": key, "state_key": state_key, "kind": kind, "who": who(r), "target": target_of(r),
                 "start": r["gone"] - r["elapsed"], "end": r["gone"]})

rows.sort(key=lambda r: r["start"])
first, seen_state, seen_full = {}, collections.defaultdict(set), set()
for r in rows:
    r["repeat"] = r["at_once"] = False
    # Same sources compiled before under any flags (check, build, test, another profile):
    # a schedule that ran one flavour per state could serve this from that run, or skip it.
    r["state_repeat"] = r["state_key"] is not None and r["state_key"] in seen_state
    # Which kind of repeat: the same unit again in the same target (Cargo threw away a fresh
    # result: fingerprint churn), the same sources under other flags in the same target, or
    # the same sources in another slot's target.
    r["why"] = ""
    if r["state_repeat"]:
        targets = seen_state[r["state_key"]]
        if (r["key"], r["target"]) in seen_full: r["why"] = "same unit, same target"
        elif r["target"] in targets: r["why"] = "other flags, same target"
        else: r["why"] = "another target"
    if r["state_key"] is not None:
        seen_state[r["state_key"]].add(r["target"]); seen_full.add((r["key"], r["target"]))
    if r["key"] is None: continue
    f = first.get(r["key"])
    if f is None: first[r["key"]] = r; continue
    if r["target"] != f["target"]:
        r["repeat"] = True
        r["at_once"] = r["start"] < f["end"]

span_h = (max(r["end"] for r in rows) - min(r["start"] for r in rows)) / 3600 if rows else 0
def tot(rs): return len(rs), sum(r["cpu"] for r in rs)
print(f"window: {span_h:.2f} h, {len(rows)} compiles seen, {len({r['target'] for r in rows})} target dirs")
print(f"{'':34}{'compiles':>9}{'cpu_s':>10}{'repeats':>9}{'rep_cpu_s':>11}{'at_once':>9}{'once_cpu_s':>11}{'same_src':>9}{'src_cpu_s':>10}")
groups = collections.defaultdict(list)
for r in rows:
    groups[("all", "")].append(r); groups[("kind", r["kind"])].append(r); groups[("who", r["who"])].append(r)
    groups[("crate", r["crate"])].append(r)
for g in sorted(groups, key=lambda g: (g[0] != "all", g[0], -sum(r["cpu"] for r in groups[g]))):
    rs = groups[g]
    if g[0] == "crate" and sum(r["cpu"] for r in rs) < 60: continue
    n, c = tot(rs); rn, rc = tot([r for r in rs if r["repeat"]]); on, oc = tot([r for r in rs if r["at_once"]])
    sn, sc = tot([r for r in rs if r["state_repeat"]])
    print(f"{(g[0] + ' ' + g[1])[:33]:34}{n:9}{c:10.0f}{rn:9}{rc:11.0f}{on:9}{oc:11.0f}{sn:9}{sc:10.0f}")
print("same sources compiled again, by kind:")
for why in ("same unit, same target", "other flags, same target", "another target"):
    rs = [r for r in rows if r["why"] == why]
    print(f"  {why:28}{len(rs):6}{sum(r['cpu'] for r in rs):10.0f} cpu_s")
unmatched = [r for r in rows if r["key"] is None]
print(f"unmatched (no state): {len(unmatched)} compiles, {sum(r['cpu'] for r in unmatched):.0f} cpu_s")

with open(os.path.join(data, "compiles.csv"), "w") as out:
    out.write("start,end,crate,kind,who,emit,test,target,cpu_s,elapsed_s,repeat,at_once,same_source,why\n")
    for r in rows:
        out.write(f"{r['start']:.0f},{r['end']:.0f},{r['crate']},{r['kind']},{r['who']},{(r.get('emit') or '').replace(',', '+')},"
                  f"{r.get('test')},{r['target']},{r['cpu']:.2f},{r['elapsed']:.0f},{r['repeat']},{r['at_once']},{r['state_repeat']},{r['why'].replace(',', ';')}\n")
