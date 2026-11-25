#!/usr/bin/env python3
"""Prints the pull request a merge queue run tests, or nothing for any other run.

    queued_pr.py "$HEAD_REF"

A queue run's branch is `gh-readonly-queue/<base>/pr-<n>-<base sha>`. When the
queue rebuilds a group, the same `<n>` comes back on a new base sha, so `native`
keys its concurrency on `<n>` and the rebuilt run cancels the abandoned one.
docs/practices/ci.md, *A rebuilt queue group cancels its old native job*.
"""
import re
import sys

QUEUE_REF = re.compile(r"^(?:refs/heads/)?gh-readonly-queue/.+/pr-([0-9]+)-[0-9a-f]+$")


def queued_pr(ref):
    found = QUEUE_REF.match(ref)
    return found.group(1) if found else ""


if __name__ == "__main__":
    print(queued_pr(sys.argv[1] if len(sys.argv) > 1 else ""))
