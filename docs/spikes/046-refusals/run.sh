#!/usr/bin/env bash
#
# One run of a yardstick Job for spike 46: the template as the plan's three groups, with plain refusals at the gates.
# Based on docs/spikes/044-seeded/run.sh; the rig's and spike 44's files are not changed.
#
#   run.sh <tag> <yardstick.md> <test-file-under-apps/desktop> [--rule REGEX::OPTION_REGEX::ANSWER]...
#
# Differences from spike 44's run.sh:
#   - nothing is seeded into the task list; the brief carries the three group titles and what ends each
#   - the gate keeper (gates.py) is the same idea and starts before the first message; it publishes its record
#     where the mod's refusals read it: $HOME/.spike046/<session id> -> the run's directory
#   - the Job runs the spike's copy of the mod (variant-046/mod/armada) through agent-wrapper.sh, for this
#     session only (the wrapper explains the claim)
#   - the run's output is runs/refusals046-<tag>-<time>/
#
# One real Job. This script starts exactly one session and never retries.

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
v="$scratch/variant-046"

runtime="$scratch/home/user/Library/Application Support/Armada/fleet.json"
pid=$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["pid"])' "$runtime" 2>/dev/null || true)
if [ -z "$pid" ] || ! kill -0 "$pid" 2>/dev/null; then
  echo "run.sh: no Fleet is up for $scratch" >&2
  exit 1
fi
port=$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["port"])' "$runtime")
api="http://127.0.0.1:$port"
out="$scratch/runs/refusals046-$tag-$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$out"
export port scratch out rig

# workflows off, so the old rail stays out (the rig's baseline does the same)
source "$rig/variants/baseline.sh"
variant_prepare

# --- the brief: the yardstick's "Brief" section, plus the plan's shape ---------
brief_line='How to plan. Record your plan with record_plan with exactly three groups, in this order, with these titles word for word: "Find the cause and plan the fix", which ends when the person approves your plan; "Fix and test", which ends when Fleet has run the Checks green at the commit you pushed to origin; and "Deliver", which ends when the branch is merged. Add your own tasks under each group. The end of each group is a gate that Fleet closes, not you. Pushing the branch belongs to "Fix and test". For "Deliver", ask the person to merge the branch with ask_the_person once it is pushed; the merge is theirs. That goes past "pushing the branch is the end of it" above.'
python3 -I - "$yard" "$out/brief.txt" "$brief_line" <<'PY'
import sys
text = open(sys.argv[1]).read()
start = text.index("## Brief") + len("## Brief")
end = text.index("## What the right answer looks like")
open(sys.argv[2], "w").write(text[start:end].strip() + "\n\n" + sys.argv[3] + "\n")
PY

# --- the spike's mod, for this session only: install the wrapper, arm the claim -
python3 -I - "$here/agent-wrapper.sh" "$scratch/bin/agent" "$v" <<'PY'
import os, shutil, sys
src, dst, v = sys.argv[1:]
for name in ("claim", "session"):
    if os.path.exists(f"{v}/{name}"):
        os.remove(f"{v}/{name}")
tmp = dst + ".046.tmp"
shutil.copyfile(src, tmp)
os.chmod(tmp, 0o755)
os.replace(tmp, dst)
open(f"{v}/claim", "w").close()
PY

body='{"manifest_id": "armada", "title": "refusals046 '"$tag"'", "model": "sonnet", "effort": "medium", "mode": "auto"}'
sid=$(curl -sf -X POST "$api/sessions/start" -H 'content-type: application/json' -d "$body" | python3 -I -c 'import json,sys; print(json.load(sys.stdin)["id"])')
echo "$sid" >"$out/session_id"
echo "run.sh: $tag, Job $sid, output $out"

# --- publish the gate record where the mod reads it, start the gate keeper -----
mkdir -p "$scratch/home/user/.spike046"
ln -sfn "$out" "$scratch/home/user/.spike046/$sid"
python3 -I "$here/gates.py" --scratch "$scratch" --session "$sid" --run-dir "$out" --test-file "$testfile" --template "$tree" --rig "$rig" --poll 1 \
  >"$out/gates.stdout" 2>&1 &
gates_pid=$!
# the record exists before the Job is told anything
for _ in 1 2 3 4 5; do [ -f "$out/record.json" ] && break; sleep 1; done

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
cp "$v/wrapper.log" "$out/wrapper.log" 2>/dev/null || true
curl -sf -X POST "$api/sessions/close" -H 'content-type: application/json' -d "{\"session_id\":\"$sid\"}" >/dev/null || true
# the claim is spent; the swap stays for this session's id alone, and the wrapper log records it
python3 -I - "$v" <<'PY'
import os, sys
if os.path.exists(f"{sys.argv[1]}/claim"):
    os.remove(f"{sys.argv[1]}/claim")
PY
python3 -I "$here/timeline.py" --scratch "$scratch" --session "$sid" --acts "$out/acts.jsonl" --run-dir "$out" --out "$out/timeline.md" | tail -3
echo "run.sh: timeline at $out/timeline.md"
