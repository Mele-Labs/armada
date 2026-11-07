#!/usr/bin/env bash
#
# One run of the yardstick Job on the dev Fleet, for a named variant.
#
#   run.sh <variant> [scratch-dir]
#
# Needs the Fleet from fleet.sh to be up. Starts a fresh Job, sends the brief in
# yardstick.md as its first message, plays the person (person.py) until the Job is
# done, quiet or past RIG_MAX_MINUTES (default 40), then writes the transcript copy,
# a verdict and the timeline under <scratch>/runs/<variant>-<time>/.
# docs/spikes/042-workflow-mechanisms-rig.md says what each file is.

set -euo pipefail

variant=${1:?usage: run.sh <variant> [scratch-dir]}
rig=$(cd "$(dirname "$0")" && pwd -P)
scratch=${2:-${RIG_SCRATCH:-/Users/nickmele/Library/Application Support/Armada/attachments/job-scratch/4e9d8cea-3d92-43de-b5cc-2cbb6d77659d/devfleet}}
max=${RIG_MAX_MINUTES:-40}
[ -f "$rig/variants/$variant.sh" ] || { echo "run.sh: no variant '$variant' in $rig/variants" >&2; exit 2; }

runtime="$scratch/home/user/Library/Application Support/Armada/fleet.json"
pid=$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["pid"])' "$runtime" 2>/dev/null || true)
if [ -z "$pid" ] || ! kill -0 "$pid" 2>/dev/null; then
  echo "run.sh: no Fleet is up for $scratch. Start one: $rig/fleet.sh '$scratch'  (run_in_background)" >&2
  exit 1
fi
port=$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["port"])' "$runtime")
api="http://127.0.0.1:$port"
out="$scratch/runs/$variant-$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$out"
export port scratch out rig

# shellcheck disable=SC1090
source "$rig/variants/$variant.sh"
variant_prepare

# --- the brief: the yardstick page's "Brief" section, verbatim ----------------
python3 -I - "$rig/yardstick.md" "$out/brief.txt" <<'PY'
import sys
text = open(sys.argv[1]).read()
start = text.index("## Brief") + len("## Brief")
end = text.index("## What the right answer looks like")
open(sys.argv[2], "w").write(text[start:end].strip() + "\n")
PY

# --- a fresh Job: model and effort set by the start, so the proposer reads neither
extra=$(variant_start_extra || true)
body=$(python3 -I - "$variant" "$extra" <<'PY'
import json, sys
body = {"manifest_id": "armada", "title": f"yardstick {sys.argv[1]}", "model": "sonnet", "effort": "medium", "mode": "auto"}
if sys.argv[2].strip():
    body.update(json.loads("{" + sys.argv[2] + "}"))
print(json.dumps(body))
PY
)
sid=$(curl -sf -X POST "$api/sessions/start" -H 'content-type: application/json' -d "$body" | python3 -I -c 'import json,sys; print(json.load(sys.stdin)["id"])')
echo "$sid" >"$out/session_id"
echo "run.sh: $variant, Job $sid, output $out"
variant_after_start "$sid"

start_epoch=$(date +%s)
python3 -I - "$sid" "$out/brief.txt" "$api" <<'PY'
import json, sys, urllib.request
sid, brief, api = sys.argv[1:]
req = urllib.request.Request(api + "/sessions/message", json.dumps({"session_id": sid, "text": open(brief).read()}).encode(), {"content-type": "application/json"})
urllib.request.urlopen(req, timeout=60).read()
PY

# --- the person, until the Job is done, quiet or out of time ------------------
reason=$(python3 -I "$rig/person.py" watch --scratch "$scratch" --session "$sid" --acts "$out/acts.jsonl" --max-minutes "$max")
echo "run.sh: stopped because: $reason ($(( ($(date +%s) - start_epoch) / 60 )) minutes)"
echo "$reason" >"$out/stopped_because"

# --- what it left: read from the scratch origin ------------------------------
"$rig/verdict.sh" "$scratch" "$sid" "$reason" >"$out/verdict.txt" 2>&1
cat "$out/verdict.txt"

# --- copy this run's transcript, close the Job, write the timeline ------------
mkdir -p "$out/transcripts"
find "$scratch/home/user/.claude/projects" -name "*$sid*" -print0 | while IFS= read -r -d '' f; do
  cp -R "$f" "$out/transcripts/$(echo "$f" | sed 's|.*/projects/||; s|/|__|g')"
done
curl -sf -X POST "$api/sessions/close" -H 'content-type: application/json' -d "{\"session_id\":\"$sid\"}" >/dev/null || true
python3 -I "$rig/timeline.py" --scratch "$scratch" --session "$sid" --acts "$out/acts.jsonl" --out "$out/timeline.md" | tail -3
echo "run.sh: timeline at $out/timeline.md"
