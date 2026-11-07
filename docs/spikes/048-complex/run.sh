#!/usr/bin/env bash
#
# The one real Job of spike 48: a complex Job, on opus, through the prototype of the recommendation.
#
#   run.sh <scratch-dir>
#
# Needs the Fleet from docs/spikes/042-rig/fleet.sh to be up for <scratch-dir>. Starts exactly ONE session and never
# retries. Differences from 046-refusals/run.sh: no mod copy and no refusals; the model is opus; the gate keeper
# (gates.py) tells the agent when a gate closes and holds all_done; the person (person.py) is scripted by rules.json.
#
# Output: <scratch>/runs/complex048-<time>/ with brief.txt, session_id, keeper.jsonl, gate_rows.jsonl, state.json,
# acts.jsonl, verdict.txt, timeline.md, stopped_because, transcripts/.

set -euo pipefail

scratch=${1:?usage: run.sh <scratch-dir>}
here=$(cd "$(dirname "$0")" && pwd -P)
rig="$here/../042-rig"
tree=$(git -C "$here" rev-parse --show-toplevel)
max=${RIG_MAX_MINUTES:-75}

runtime="$scratch/home/user/Library/Application Support/Armada/fleet.json"
pid=$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["pid"])' "$runtime" 2>/dev/null || true)
if [ -z "$pid" ] || ! kill -0 "$pid" 2>/dev/null; then
  echo "run.sh: no Fleet is up for $scratch" >&2
  exit 1
fi
port=$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["port"])' "$runtime")
api="http://127.0.0.1:$port"
out="$scratch/runs/complex048-$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$out"
export port scratch out rig

# workflows off, so the old rail stays out
source "$rig/variants/baseline.sh"
variant_prepare

python3 -I - "$here/problem.md" "$out/brief.txt" <<'PY'
import sys
text = open(sys.argv[1]).read()
start = text.index("## Brief") + len("## Brief")
end = text.index("## What the right answer looks like")
open(sys.argv[2], "w").write(text[start:end].strip() + "\n")
PY

body='{"manifest_id": "armada", "title": "complex048 vs8t", "model": "opus", "effort": "medium", "mode": "auto"}'
sid=$(curl -sf -X POST "$api/sessions/start" -H 'content-type: application/json' -d "$body" | python3 -I -c 'import json,sys; print(json.load(sys.stdin)["id"])')
echo "$sid" >"$out/session_id"
echo "run.sh: Job $sid, output $out"

python3 -I "$here/gates.py" --scratch "$scratch" --session "$sid" --run-dir "$out" --template "$tree" --rig "$rig" --poll 1 \
  >"$out/gates.stdout" 2>&1 &
gates_pid=$!
echo "$gates_pid" >"$out/gates.pid"
for _ in 1 2 3 4 5; do [ -f "$out/keeper.jsonl" ] && break; sleep 1; done

start_epoch=$(date +%s)
echo "$start_epoch" >"$out/start_epoch"
python3 -I - "$sid" "$out/brief.txt" "$api" <<'PY'
import json, sys, urllib.request
sid, brief, api = sys.argv[1:]
req = urllib.request.Request(api + "/sessions/message", json.dumps({"session_id": sid, "text": open(brief).read()}).encode(), {"content-type": "application/json"})
urllib.request.urlopen(req, timeout=60).read()
PY

reason=$(python3 -I "$here/person.py" watch --scratch "$scratch" --session "$sid" --acts "$out/acts.jsonl" --run-dir "$out" --rules "$here/rules.json" --max-minutes "$max")
echo "run.sh: stopped because: $reason ($(( ($(date +%s) - start_epoch) / 60 )) minutes)"
echo "$reason" >"$out/stopped_because"

touch "$out/gates.stop"
wait "$gates_pid" || true

# --- what it left, read from the scratch origin ----------------------------
set +e
{
  branch="armada/session-${sid:0:8}"
  origin="$scratch/origin.git"
  echo "stopped because: $reason"
  echo "refs on origin:"
  git --git-dir="$origin" for-each-ref --format='  %(refname:short) %(objectname:short)' refs/heads
  if git --git-dir="$origin" rev-parse -q --verify "refs/heads/$branch" >/dev/null; then
    echo "commits on $branch past the base: $(git --git-dir="$origin" rev-list --count "$(git --git-dir="$origin" rev-list --max-parents=0 refs/heads/main | tail -1)..refs/heads/$branch" 2>/dev/null)"
    base=$(git --git-dir="$origin" merge-base "refs/heads/$branch" "$(git --git-dir="$origin" rev-parse refs/heads/main^1 2>/dev/null || echo refs/heads/main)" 2>/dev/null || true)
    echo "files changed by the branch against its merge base with main (before the merge):"
    git --git-dir="$origin" diff --stat "${base:-refs/heads/main}" "refs/heads/$branch" | sed 's/^/  /'
  fi
} >"$out/verdict.txt" 2>&1
set -e
cat "$out/verdict.txt"

mkdir -p "$out/transcripts"
find "$scratch/home/user/.claude/projects" -name "*$sid*" -print0 | while IFS= read -r -d '' f; do
  cp -R "$f" "$out/transcripts/$(echo "$f" | sed 's|.*/projects/||; s|/|__|g')"
done
curl -sf "$api/sessions/one?session_id=$sid" >"$out/session_one.json" || true
curl -sf -X POST "$api/sessions/close" -H 'content-type: application/json' -d "{\"session_id\":\"$sid\"}" >/dev/null || true
python3 -I "$here/timeline.py" --scratch "$scratch" --session "$sid" --acts "$out/acts.jsonl" --run-dir "$out" --out "$out/timeline.md" | tail -3
echo "run.sh: timeline at $out/timeline.md"
