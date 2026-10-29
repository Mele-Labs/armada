#!/usr/bin/env python3
"""Sorts the keys `armada covers` prints into the jobs that run them.

    armada covers < paths | plan.py      JSON on stdout, exit 1 on a key nothing runs
    armada covers < paths | plan.py --quick    the same, keeping only QUICK to run
    ... | plan.py --quick --rerun failed.txt   and the Checks of the job names
                                               the merge queue last failed

A root Check is a bare name and has a static job in checks.yml. A workspace
Check is `<dir>:<name>`: the desktop tests that read no surface run in one Linux
job, and the other Node Checks
run in one matrix job that calls `armada check <key>`. A key that is
neither run nor named in EXCLUDED fails the plan, so a Check added to a manifest
is never skipped for want of a line here. docs/practices/ci.md, *How a Check is
chosen*.
"""
import json
import re
import sys

# Bare names with a static job in checks.yml.
ROOT = {"build", "test", "acceptance", "typecheck", "hooks_test", "preview_test", "sync_mod_test"}

# Run by the Linux job that runs vitest directly, outside the matrix so that
# `desktop_test` reports it with the shards and `ci` does not wait on it.
DESKTOP = {"apps/desktop/unit:desktop_test"}

# Run by a static Rust job: a workspace Check whose command is cargo, not Node.
RUST = {"apps/desktop:xtask_test"}

# Workspace Checks by name that the Linux matrix job runs.
MATRIX = {"typecheck", "bridge_build", "storybook", "components_test", "screens_test", "pocket_test", "store_test", "test"}

# Of those, the ones that never open a browser.
NO_BROWSER = {"typecheck", "bridge_build", "pocket_test", "store_test"}

# Keys CI deliberately does not run, and why.
EXCLUDED = {}

# Check names a pull request's own run keeps with `--quick`: the ones that
# compile. The merge queue runs every Check, so the pull request run is an early
# warning and the queue is where the suite runs once. Owner, 17 Oct 2026.
QUICK = {"build", "typecheck", "bridge_build"}

# Job names in checks.yml that are not their Check's key. A shard's name is its
# Check's with ` <n>/<m>` after it.
JOB_KEYS = {"desktop_unit": "apps/desktop/unit:desktop_test"}


def rerun_keys(job_names):
    """The Check keys of the jobs a merge queue run failed. A pull request the
    queue dropped keeps these with `--quick`, so a fix is tried on its own run
    before it waits in the queue again."""
    keys = set()
    for name in job_names:
        name = re.sub(r" \d+/\d+$", "", name.strip())
        keys.add(JOB_KEYS.get(name, name))
    return keys


def plan(keys, quick=False, rerun=frozenset()):
    run, matrix, excluded, unrun = [], [], [], []
    for key in dict.fromkeys(keys):
        name = key.rsplit(":", 1)[-1]
        if key in EXCLUDED:
            excluded.append(key)
        elif ":" not in key and key in ROOT:
            run.append(key)
        elif key in DESKTOP or key in RUST:
            run.append(key)
        elif ":" in key and name in MATRIX:
            run.append(key)
            matrix.append({"key": key, "browsers": str(name not in NO_BROWSER).lower()})
        else:
            unrun.append(key)
    queued = []
    if quick:
        queued = [key for key in run if key.rsplit(":", 1)[-1] not in QUICK and key not in rerun]
        run = [key for key in run if key not in queued]
        matrix = [row for row in matrix if row["key"] in run]
    rerun = [key for key in run if key in rerun] if quick else []
    return {"checks": run, "matrix": {"include": matrix}, "excluded": excluded, "unrun": unrun, "queued": queued, "rerun": rerun}


def main():
    keys = [line.strip() for line in sys.stdin if line.strip()]
    args = sys.argv[1:]
    rerun = set()
    if "--rerun" in args:
        with open(args[args.index("--rerun") + 1]) as failed:
            rerun = rerun_keys(line for line in failed if line.strip())
    answer = plan(keys, quick="--quick" in args, rerun=rerun)
    if answer["unrun"]:
        for key in answer["unrun"]:
            print(f"::error::`armada covers` names {key}, which CI neither runs nor excludes: add it to .github/ci/plan.py", file=sys.stderr)
        return 1
    for key in answer["excluded"]:
        print(f"excluded {key}: {EXCLUDED[key]}", file=sys.stderr)
    print(json.dumps(answer, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    sys.exit(main())
