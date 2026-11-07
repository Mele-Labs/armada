#!/usr/bin/env bash
#
# Exercises the person's plan approval and a question answer on a Job that edits nothing:
# the Job is told to record a one-task plan, wait for approval, then ask one question and
# call all_done. Costs a few cents. Prints the acts person.py took.
#
#   check-acts.sh [scratch-dir]

set -euo pipefail
rig=$(cd "$(dirname "$0")" && pwd -P)
scratch=${1:-${RIG_SCRATCH:-/Users/nickmele/Library/Application Support/Armada/attachments/job-scratch/4e9d8cea-3d92-43de-b5cc-2cbb6d77659d/devfleet}}
runtime="$scratch/home/user/Library/Application Support/Armada/fleet.json"
api="http://127.0.0.1:$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["port"])' "$runtime")"
out="$scratch/runs/check-acts-$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$out"

sid=$(curl -sf -X POST "$api/sessions/start" -H 'content-type: application/json' \
  -d '{"manifest_id":"armada","title":"check person acts","model":"sonnet","effort":"low","mode":"auto"}' |
  python3 -I -c 'import json,sys; print(json.load(sys.stdin)["id"])')
python3 -I - "$sid" "$api" <<'PY'
import json, sys, urllib.request
sid, api = sys.argv[1:]
text = (
    "Do not edit, commit or run any shell command. Step 1: call record_plan with a summary and one group "
    "(title 'Check', agent 'explorer', paths ['docs'], one task 'Read the README'). Step 2: when the plan is approved, "
    "ask me one question with the ask tool: 'Which colour?' with options Red and Blue. Step 3: when I answer, call all_done."
)
req = urllib.request.Request(api + "/sessions/message", json.dumps({"session_id": sid, "text": text}).encode(), {"content-type": "application/json"})
urllib.request.urlopen(req, timeout=60).read()
PY
python3 -I "$rig/person.py" watch --scratch "$scratch" --session "$sid" --acts "$out/acts.jsonl" --max-minutes 6 --quiet-seconds 90
cat "$out/acts.jsonl"
python3 -I "$rig/timeline.py" --scratch "$scratch" --session "$sid" --acts "$out/acts.jsonl" --out "$out/timeline.md" | cut -c1-220
curl -sf -X POST "$api/sessions/close" -H 'content-type: application/json' -d "{\"session_id\":\"$sid\"}" >/dev/null || true
