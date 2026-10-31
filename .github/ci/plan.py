#!/usr/bin/env python3
"""Sorts the keys `armada covers` prints into the jobs that run them.

    armada covers < paths | plan.py      JSON on stdout, exit 1 on a key nothing runs
    armada covers < paths | plan.py --quick    the same, keeping only QUICK to run
    ... | plan.py --quick --rerun failed.txt   and the Checks of the job names
                                               the merge queue last failed

With `--quick`, `compile` names the Rust Checks left to the queue whose test
code the pull request compiles all the same.

A root Check is a bare name and has a static job in checks.yml. A workspace
Check is `<dir>:<name>`: the desktop tests that read no surface run in one Linux
job, and the other Node Checks run in a few grouped matrix jobs, each calling
`armada check <key>` for its keys one after another. A key that is
neither run nor named in EXCLUDED fails the plan, so a Check added to a manifest
is never skipped for want of a line here. docs/practices/ci.md, *How a Check is
chosen*.
"""
import hashlib
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

# The grouped jobs the matrix keys run in, one runner each: a runner's checkout
# and setup cost about as much as a short Check, so a job runs several Checks
# one after another. The merge queue builds five runs at once, so fewer jobs
# means less waiting for a runner. Names are the job names.
NODE = "node checks"
COMPONENTS = "components"
SURFACES = ("surface tests 1/2", "surface tests 2/2")
GROUPS = [NODE, COMPONENTS, *SURFACES]

# Of the matrix names, the ones that run in the components job: the slowest
# (components_test) and the one that shares its build (storybook).
COMPONENT_NAMES = {"components_test", "storybook"}

# Keys CI deliberately does not run, and why.
EXCLUDED = {}

# Check names a pull request's own run keeps with `--quick`: the ones that
# compile. The merge queue runs every Check, so the pull request run is an early
# warning and the queue is where the suite runs once. Owner, 17 Oct 2026.
QUICK = {"build", "typecheck", "bridge_build"}

# Root Checks whose test code a pull request's run compiles without running it,
# when the queue is left to run them: `cargo build` compiles no test target, so
# a change that broke a test's code first failed in the queue, 4 of 16 failed
# queue runs on 30 and 31 Oct 2026.
COMPILE = {"test", "acceptance"}

# Job names in checks.yml that are not their Check's key. A shard's name is its
# Check's with ` <n>/<m>` after it.
JOB_KEYS = {"desktop_unit": "apps/desktop/unit:desktop_test"}


def group_of(key):
    """The grouped job a matrix key runs in. The surface tests split by a hash
    of the key, so a key stays in its half as others come and go. Today's eight
    fall four and four."""
    name = key.rsplit(":", 1)[-1]
    if name in NO_BROWSER:
        return NODE
    if name in COMPONENT_NAMES:
        return COMPONENTS
    return SURFACES[hashlib.sha256(key.encode()).digest()[0] % len(SURFACES)]


def rerun_keys(job_names):
    """What a merge queue run failed, as Check keys and group names. A pull
    request the queue dropped keeps these with `--quick`, so a fix is tried on
    its own run before it waits in the queue again. A grouped job does not say
    which of its keys failed, so its name stands for the keys `plan` puts in it;
    `plan` keeps the ones the pull request's own change names."""
    keys = set()
    for name in job_names:
        name = name.strip()
        if name not in GROUPS:
            name = re.sub(r" \d+/\d+$", "", name)
        keys.add(JOB_KEYS.get(name, name))
    return keys


def plan(keys, quick=False, rerun=frozenset()):
    run, grouped, excluded, unrun = [], [], [], []
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
            grouped.append(key)
        else:
            unrun.append(key)
    queued, compile = [], []
    again = lambda key: key in rerun or (key in grouped and group_of(key) in rerun)
    if quick:
        queued = [key for key in run if key.rsplit(":", 1)[-1] not in QUICK and not again(key)]
        run = [key for key in run if key not in queued]
        compile = [key for key in queued if key in COMPILE]
    rerun = [key for key in run if again(key)] if quick else []
    include = []
    for group in GROUPS:
        keys = [key for key in run if key in grouped and group_of(key) == group]
        if keys:
            # A browser opens in every group but the node one.
            include.append({"group": group, "keys": " ".join(keys), "browsers": str(group != NODE).lower()})
    return {"checks": run, "matrix": {"include": include}, "excluded": excluded, "unrun": unrun, "queued": queued, "rerun": rerun, "compile": compile}


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
