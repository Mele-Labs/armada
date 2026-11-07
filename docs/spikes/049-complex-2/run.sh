#!/usr/bin/env bash
#
# The one real Job of spike 49: a complex Job with subagents, on opus, through the fixed prototype of the recommendation.
#
#   run.sh start  <scratch-dir>            makes the run folder, starts ONE Job, prints "<out-dir> <session-id>"
#   keeper and person are then started by the caller, each as a background task it holds:
#     python3 -I gates.py  --scratch S --session ID --run-dir OUT --template TREE --rig ../042-rig --poll 1
#     python3 -I person.py watch --scratch S --session ID --acts OUT/acts.jsonl --run-dir OUT --rules rules.json --max-minutes 75
#   run.sh send   <scratch-dir> <out-dir>   sends the brief as the Job's first message
#   run.sh finish <scratch-dir> <out-dir>   after the person's script ended: verdict, transcripts, close, timeline
#
# Needs the Fleet from fleet.sh to be up for <scratch-dir>. Never retries a start. Workflows off. Model opus.

set -euo pipefail

mode=${1:?usage: run.sh start|send|finish <scratch-dir> [out-dir]}
scratch=${2:?scratch dir}
here=$(cd "$(dirname "$0")" && pwd -P)
rig="$here/../042-rig"
runtime="$scratch/home/user/Library/Application Support/Armada/fleet.json"
pid=$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["pid"])' "$runtime" 2>/dev/null || true)
if [ -z "$pid" ] || ! kill -0 "$pid" 2>/dev/null; then
  echo "run.sh: no Fleet is up for $scratch" >&2
  exit 1
fi
port=$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["port"])' "$runtime")
api="http://127.0.0.1:$port"

case "$mode" in
start)
  out="$scratch/runs/complex049-$(date -u +%Y%m%dT%H%M%SZ)"
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
  body='{"manifest_id": "armada", "title": "complex049 vs8t and 20v4", "model": "opus", "effort": "medium", "mode": "auto"}'
  sid=$(curl -sf -X POST "$api/sessions/start" -H 'content-type: application/json' -d "$body" | python3 -I -c 'import json,sys; print(json.load(sys.stdin)["id"])')
  echo "$sid" >"$out/session_id"
  echo "$out $sid"
  ;;
send)
  out=${3:?out dir}
  sid=$(cat "$out/session_id")
  date +%s >"$out/start_epoch"
  python3 -I - "$sid" "$out/brief.txt" "$api" <<'PY'
import json, sys, urllib.request
sid, brief, api = sys.argv[1:]
req = urllib.request.Request(api + "/sessions/message", json.dumps({"session_id": sid, "text": open(brief).read()}).encode(), {"content-type": "application/json"})
urllib.request.urlopen(req, timeout=60).read()
PY
  echo "run.sh: brief sent to $sid"
  ;;
finish)
  out=${3:?out dir}
  sid=$(cat "$out/session_id")
  set +e
  {
    branch="armada/session-${sid:0:8}"
    origin="$scratch/origin.git"
    echo "stopped because: $(cat "$out/stopped_because" 2>/dev/null)"
    echo "refs on origin:"
    git --git-dir="$origin" for-each-ref --format='  %(refname:short) %(objectname:short)' refs/heads
    if git --git-dir="$origin" rev-parse -q --verify "refs/heads/$branch" >/dev/null; then
      base=$(git --git-dir="$origin" rev-list --max-parents=0 refs/heads/main | tail -1)
      echo "commits on $branch past the first commit of main: $(git --git-dir="$origin" rev-list --count "$base..refs/heads/$branch" 2>/dev/null)"
      mb=$(git --git-dir="$origin" merge-base "refs/heads/$branch" "$(git --git-dir="$origin" rev-parse refs/heads/main^1 2>/dev/null || echo refs/heads/main)" 2>/dev/null || true)
      echo "files changed by the branch against its merge base with main (before the merge):"
      git --git-dir="$origin" diff --stat "${mb:-refs/heads/main}" "refs/heads/$branch" | sed 's/^/  /'
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
  ;;
*)
  echo "run.sh: unknown mode $mode" >&2
  exit 2
  ;;
esac
