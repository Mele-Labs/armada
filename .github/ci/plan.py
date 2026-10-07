#!/usr/bin/env python3
"""Sorts the keys `armada covers` prints into the jobs that run them.

    armada covers < paths | plan.py      JSON on stdout, exit 1 on a key nothing runs

A root Check is a bare name and has a static job in checks.yml. A workspace
Check is `<dir>:<name>`: the desktop tests run in their own sharded job, and the
Node Checks run in one matrix job that calls `armada check <key>`. A key that is
neither run nor named in EXCLUDED fails the plan, so a Check added to a manifest
is never skipped for want of a line here. docs/practices/ci.md, *How a Check is
chosen*.
"""
import json
import sys

# Bare names with a static job in checks.yml.
ROOT = {"build", "test", "acceptance", "typecheck", "format", "hooks_test", "preview_test"}

# Run by the macOS shard job, which cannot go through `armada check`: that has
# no `--shard`.
SHARDED = {"apps/desktop:desktop_test"}

# Run by a static Rust job: a workspace Check whose command is cargo, not Node.
RUST = {"apps/desktop:xtask_test"}

# Workspace Checks by name that the Linux matrix job runs.
MATRIX = {"typecheck", "bridge_build", "storybook", "components_test", "screens_test", "test"}

# Of those, the ones that never open a browser.
NO_BROWSER = {"typecheck", "bridge_build"}

# Keys CI deliberately does not run, and why.
EXCLUDED = {}


def plan(keys):
    run, matrix, excluded, unrun = [], [], [], []
    for key in dict.fromkeys(keys):
        name = key.rsplit(":", 1)[-1]
        if key in EXCLUDED:
            excluded.append(key)
        elif ":" not in key and key in ROOT:
            run.append(key)
        elif key in SHARDED or key in RUST:
            run.append(key)
        elif ":" in key and name in MATRIX:
            run.append(key)
            matrix.append({"key": key, "browsers": str(name not in NO_BROWSER).lower()})
        else:
            unrun.append(key)
    return {"checks": run, "matrix": {"include": matrix}, "excluded": excluded, "unrun": unrun}


def main():
    keys = [line.strip() for line in sys.stdin if line.strip()]
    answer = plan(keys)
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
