#!/usr/bin/env bash
#
# One run of a yardstick Job with the bug workflow seeded into the agent's own task list.
# Based on docs/spikes/042-rig/run.sh for spike 44; the rig's files are not changed.
#
#   run.sh <tag> <yardstick.md> <test-file-under-apps/desktop> [--rule REGEX::OPTION_REGEX::ANSWER]...
#
# Differences from the rig's run.sh:
#   - the brief is the yardstick's "Brief" section plus one line (brief_line below)
#   - after the Job starts and before its first message, seed.py writes the template into the Job's
#     task list, and gates.py (Fleet's gate keeper) starts; gates.stop ends it once the person is done
#   - person.py merges the branch when asked
#   - the run's output is runs/seeded044-<tag>-<time>/

set -euo pipefail

tag=${1:?usage: run.sh <tag> <yardstick.md> <test-file> [--rule ...]}
yard=${2:?yardstick.md}
testfile=${3:?test file relative to apps/desktop}
shift 3
scratch=${RIG_SCRATCH:-/Users/nickmele/Library/Application Support/Armada/attachments/job-scratch/4e9d8cea-3d92-43de-b5cc-2cbb6d77659d/devfleet}
rules=()
while [ $# -gt 0 ]; do
  if [ "$1" = "--rule" ]; then rules+=(--rule "$2"); shift 2; else echo "run.sh: unexpected $1" >&2; exit 2; fi
done

here=$(cd "$(dirname "$0")" && pwd -P)
rig="$here/../042-rig"
tree=$(git -C "$here" rev-parse --show-toplevel)
max=${RIG_MAX_MINUTES:-40}

runtime="$scratch/home/user/Library/Application Support/Armada/fleet.json"
pid=$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["pid"])' "$runtime" 2>/dev/null || true)
if [ -z "$pid" ] || ! kill -0 "$pid" 2>/dev/null; then
  echo "run.sh: no Fleet is up for $scratch" >&2
  exit 1
fi
port=$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["port"])' "$runtime")
api="http://127.0.0.1:$port"
out="$scratch/runs/seeded044-$tag-$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$out"
export port scratch out rig

# workflows off, so the old rail stays out (the rig's baseline does the same)
source "$rig/variants/baseline.sh"
variant_prepare

# --- the brief: the yardstick's "Brief" section, plus one line ----------------
brief_line='Your task list holds the workflow for this Job: work its tasks in order. A task that starts "Gate:" is Fleet'"'"'s to tick, not yours. The list also asks you to have the person merge the branch, which goes past "pushing the branch is the end of it" above.'
python3 -I - "$yard" "$out/brief.txt" "$brief_line" <<'PY'
import sys
text = open(sys.argv[1]).read()
start = text.index("## Brief") + len("## Brief")
end = text.index("## What the right answer looks like")
open(sys.argv[2], "w").write(text[start:end].strip() + "\n\n" + sys.argv[3] + "\n")
PY

body='{"manifest_id": "armada", "title": "seeded044 '"$tag"'", "model": "sonnet", "effort": "medium", "mode": "auto"}'
sid=$(curl -sf -X POST "$api/sessions/start" -H 'content-type: application/json' -d "$body" | python3 -I -c 'import json,sys; print(json.load(sys.stdin)["id"])')
echo "$sid" >"$out/session_id"
echo "run.sh: $tag, Job $sid, output $out"

# --- seed the template and start the gate keeper, before the first message ----
python3 -I "$here/seed.py" --bug-json "$tree/.armada/workflows/bug.json" --home "$scratch/home/user" --session "$sid" --run-dir "$out"
python3 -I "$here/gates.py" --scratch "$scratch" --session "$sid" --run-dir "$out" --test-file "$testfile" --template "$tree" --rig "$rig" \
  >"$out/gates.stdout" 2>&1 &
gates_pid=$!

start_epoch=$(date +%s)
python3 -I - "$sid" "$out/brief.txt" "$api" <<'PY'
import json, sys, urllib.request
sid, brief, api = sys.argv[1:]
req = urllib.request.Request(api + "/sessions/message", json.dumps({"session_id": sid, "text": open(brief).read()}).encode(), {"content-type": "application/json"})
urllib.request.urlopen(req, timeout=60).read()
PY

# --- the person, until the Job is done, quiet or out of time ------------------
reason=$(python3 -I "$here/person.py" watch --scratch "$scratch" --session "$sid" --acts "$out/acts.jsonl" --max-minutes "$max" ${rules[@]+"${rules[@]}"})
echo "run.sh: stopped because: $reason ($(( ($(date +%s) - start_epoch) / 60 )) minutes)"
echo "$reason" >"$out/stopped_because"

touch "$out/gates.stop"
wait "$gates_pid" || true

# --- what it left -------------------------------------------------------------
VERDICT_TEST="$testfile" "$here/verdict.sh" "$scratch" "$sid" "$reason" >"$out/verdict.txt" 2>&1
cat "$out/verdict.txt"

mkdir -p "$out/transcripts"
find "$scratch/home/user/.claude/projects" -name "*$sid*" -print0 | while IFS= read -r -d '' f; do
  cp -R "$f" "$out/transcripts/$(echo "$f" | sed 's|.*/projects/||; s|/|__|g')"
done
mkdir -p "$out/tasks-final" && cp "$scratch/home/user/.claude/tasks/$sid/"*.json "$out/tasks-final/" 2>/dev/null || true
curl -sf -X POST "$api/sessions/close" -H 'content-type: application/json' -d "{\"session_id\":\"$sid\"}" >/dev/null || true
python3 -I "$here/timeline.py" --scratch "$scratch" --session "$sid" --acts "$out/acts.jsonl" --run-dir "$out" --out "$out/timeline.md" | tail -3
echo "run.sh: timeline at $out/timeline.md"
