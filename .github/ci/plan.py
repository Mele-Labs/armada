#!/usr/bin/env python3
"""Sorts the keys `armada covers` prints into the jobs that run them.

    armada covers < paths | plan.py      JSON on stdout, exit 1 on a key nothing runs
    armada covers < paths | plan.py --quick    the same, keeping only QUICK to run
    ... | plan.py --quick --rerun failed.txt   and the Checks of the job names
                                               the merge queue last failed

With `--quick`, `compile` names the Rust Checks left to the queue whose test
code the pull request compiles all the same.

A root Check is a bare name and has a static job in checks.yml. A workspace
Check is `<dir>:<name>`, and the Node Checks run in one grouped matrix job, calling
`armada check <key>` for its keys one after another. A key that is
neither run nor named in EXCLUDED fails the plan, so a Check added to a manifest
is never skipped for want of a line here. docs/practices/ci.md, *How a Check is
chosen*.
"""
import json
import re
import sys

# Bare names with a static job in checks.yml.
ROOT = {"build", "test", "acceptance", "typecheck", "xtask_test", "hooks_test", "preview_test", "sync_mod_test", "keep_build_test", "format_catch_up_test"}

# Workspace Checks by name that the Linux matrix job runs.
MATRIX = {"typecheck", "pocket_test", "store_test", "bundle_current", "test"}

# The grouped job the matrix keys run in: a runner's checkout and setup cost
# about as much as a short Check, so one job runs several Checks one after
# another. The name is the job's.
NODE = "node checks"
GROUPS = [NODE]

# Keys CI deliberately does not run, and why.
EXCLUDED = {
    "apps/pocket:format_check": "the `format` job runs it on every change, so a Job's pre-PR Check would repeat it",
    "crates/bridge-native:shot_check": "it compares a Job's own prototype scene, which lives in the Job's scratch folder and which no CI runner has",
    "apps/store:format_check": "the `format` job runs it on every change, so a Job's pre-PR Check would repeat it",
}

# The native Bridge's tests: a workspace Check of a crate gpui makes buildable on macOS only, so
# no Linux job runs it. The `native` job does, on any run whose change reaches the crate, and
# `plan` says so in `native` rather than in `checks`. A pull request's `--quick` run keeps it
# too: the merge queue is not the first place a native test fails.
NATIVE = "crates/bridge-native:test"

# Check names a pull request's own run keeps with `--quick`: the ones that
# compile. The merge queue runs every Check, so the pull request run is an early
# warning and the queue is where the suite runs once. Owner, 17 Oct 2026.
QUICK = {"build", "typecheck"}

# Root Checks whose test code a pull request's run compiles without running it,
# when the queue is left to run them: `cargo build` compiles no test target, so
# a change that broke a test's code first failed in the queue, 4 of 16 failed
# queue runs on 30 and 31 Oct 2026.
COMPILE = {"test", "acceptance"}

# A shard's job name is its Check's with ` <n>/<m>` after it.


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
        keys.add(name)
    return keys


def plan(keys, quick=False, rerun=frozenset()):
    run, grouped, excluded, unrun = [], [], [], []
    native = NATIVE in keys
    for key in dict.fromkeys(keys):
        name = key.rsplit(":", 1)[-1]
        if key == NATIVE:
            continue
        if key in EXCLUDED:
            excluded.append(key)
        elif ":" not in key and key in ROOT:
            run.append(key)
        elif ":" in key and name in MATRIX:
            run.append(key)
            grouped.append(key)
        else:
            unrun.append(key)
    queued, compile = [], []
    again = lambda key: key in rerun or (key in grouped and NODE in rerun)
    if quick:
        queued = [key for key in run if key.rsplit(":", 1)[-1] not in QUICK and not again(key)]
        run = [key for key in run if key not in queued]
        compile = [key for key in queued if key in COMPILE]
    rerun = [key for key in run if again(key)] if quick else []
    include = []
    for group in GROUPS:
        keys = [key for key in run if key in grouped]
        if keys:
            include.append({"group": group, "keys": " ".join(keys)})
    return {"checks": run, "matrix": {"include": include}, "excluded": excluded, "unrun": unrun, "queued": queued, "rerun": rerun, "compile": compile, "native": native}


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
