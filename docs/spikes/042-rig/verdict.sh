#!/usr/bin/env bash
#
# What a run left, read from the scratch origin and not from the Job's slot, because
# `all_done` gives the slot back before anyone can look in it.
#
#   verdict.sh <scratch> <session-id> <stopped-because>
#
# Prints the branch, the files changed against main, and the yardstick test run at the
# pushed commit in a throwaway clone (node_modules copied on write, one vitest file).

set -uo pipefail
scratch=$1
sid=$2
reason=${3:-unknown}
rig=$(cd "$(dirname "$0")" && pwd -P)
branch="armada/session-${sid:0:8}"
origin="$scratch/origin.git"

echo "stopped because: $reason"
echo "branch: $branch"
head=$(git --git-dir="$origin" rev-parse --short "refs/heads/$branch" 2>/dev/null || true)
if [ -z "$head" ]; then
  echo "origin has the branch: no"
  exit 0
fi
echo "origin has the branch: yes, at $head"
echo "commits on it past main: $(git --git-dir="$origin" rev-list --count "refs/heads/main..refs/heads/$branch")"
echo "files changed against main:"
git --git-dir="$origin" diff --stat "refs/heads/main" "refs/heads/$branch" | sed 's/^/  /'

check="$scratch/verify-${sid:0:8}"
if [ ! -d "$check" ]; then
  git clone --quiet --local --branch "$branch" "$origin" "$check"
  (cd "$check" && bash "$rig/seed-node-modules.sh" "$(git -C "$rig" rev-parse --show-toplevel)" >/dev/null)
fi
echo "scenario.test.ts at the pushed commit:"
(cd "$check/apps/desktop" && node_modules/.bin/vitest run src/renderer/src/mock/scenario.test.ts 2>&1 | grep -E "Tests |Test Files|FAIL" | sed 's/^/  /')
