"""Turn the per-run logs in this folder into runs.csv.

A log is `<set>-<tree>-r<rep>-<step>.log`, plus `.a2.log`, `.a3.log` ... when the Check was killed
at its 900 s budget and run again on the same warm-ish target; the attempts of one step are summed.
The header lines say when the run was started, when Fleet let the Check begin (the first cargo in the
process table, `watch.py`) and when it ended; `armada check` prints nothing until it ends, so the
output itself carries no times.
"""
import csv, glob, os, re, collections

here = os.path.dirname(os.path.abspath(__file__))
os.chdir(here)

name_re = re.compile(r"^(?P<set>[A-Za-z0-9]+)-(?P<tree>.+)-r(?P<rep>\d+)-(?P<step>build|test)(?:\.a(?P<att>\d+))?\.log$")
time_re = re.compile(r"^\s*([\d.]+) real\s+([\d.]+) user\s+([\d.]+) sys")
nextest_re = re.compile(r"Summary \[[^\]]*\]\s+(\d+)(?:/\d+)? tests? run:\s+(\d+) passed(?:[^,]*)?(?:,\s+(\d+) failed)?")

groups = collections.defaultdict(list)
for f in sorted(glob.glob("*.log")):
    m = name_re.match(f)
    if m:
        groups[(m["set"], m["tree"], int(m["rep"]), m["step"])].append((int(m["att"] or 1), f))

rows = []
for (st, tree, rep, step), atts in sorted(groups.items()):
    wall = user = sys_ = queue = total_real = 0.0
    compiled = compiled_ws = tests_run = tests_failed = killed = 0
    load1 = load15 = sha = exit_ = ""
    for att, f in sorted(atts):
        t_start = t_run = t_end = None
        for ln in open(f, errors="replace").read().splitlines():
            if ln.startswith("# load_before") and att == 1:
                p = ln.split(); load1, load15 = p[2], p[4]
            elif ln.startswith("# start "):
                m = re.search(r"# start ([\d.]+) tree=(\w+)", ln); t_start = float(m[1]); sha = sha or m[2]
            elif ln.startswith("# run_started"):
                v = ln.split()[2]; t_run = None if v == "none" else float(v)
            elif ln.startswith("# ended"):
                t_end = float(ln.split()[2])
            elif ln.startswith("# exit"):
                exit_ = ln.split()[2]
            elif ln.startswith("#"):
                pass
            else:
                m = time_re.match(ln)
                if m:
                    real = float(m[1]); total_real += real; user += float(m[2]); sys_ += float(m[3])
                if re.match(r"^\s+Compiling ", ln):
                    compiled += 1
                    if "(/" in ln:
                        compiled_ws += 1
                if "budget and was killed" in ln:
                    killed += 1
                m = nextest_re.search(ln)
                if m:
                    tests_run += int(m[1]); tests_failed += int(m[3] or 0)
        if t_end is not None and t_run is not None:
            wall += t_end - t_run
            queue += max(0.0, t_run - t_start)
        elif t_end is not None:
            wall += t_end - t_start  # run start not seen: queue wait is inside this
    rows.append(dict(set=st, tree=tree, rep=rep, step=step, wall_s=round(wall, 1), user_s=round(user, 2),
                     sys_s=round(sys_, 2), crates_compiled=compiled, tests_run=tests_run,
                     tests_failed=tests_failed, load1=load1, load15=load15, sha=sha,
                     queue_wait_s=round(queue, 1), attempts=len(atts), budget_kills=killed,
                     exit=exit_, workspace_crates_compiled=compiled_ws, total_real_s=round(total_real, 1)))

cols = ["set", "tree", "rep", "step", "wall_s", "user_s", "sys_s", "crates_compiled", "tests_run",
        "tests_failed", "load1", "load15", "sha", "queue_wait_s", "attempts", "budget_kills", "exit",
        "workspace_crates_compiled", "total_real_s"]
with open("runs.csv", "w", newline="") as fh:
    w = csv.DictWriter(fh, fieldnames=cols)
    w.writeheader()
    w.writerows(rows)
print(len(rows), "rows")
