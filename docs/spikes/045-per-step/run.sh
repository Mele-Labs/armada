#!/usr/bin/env bash
#
# Spike 45: one agent per stretch of work, on the dev Fleet from ../042-rig/fleet.sh.
#
#   run.sh <tyl|35xp> <perstep|single> [scratch-dir]
#
# perstep: three fresh Jobs on one branch, each started once the gate before it has closed.
#   stretch 1  find the cause, plan the fix           gate: the person approves the plan
#   stretch 2  fix, test red then green, push         gate: the yardstick's vitest file passes at the pushed commit
#   stretch 3  deliver: merge main in, say what landed gate: the person merges the branch into the origin's main
# single: one Job with the whole yardstick brief, the rig's baseline shape, for the second yardstick.
#
# Copied from ../042-rig/run.sh and changed; the rig's files are used where they need no change
# (timeline.py, seed-node-modules.sh). Output: <scratch>/runs/perstep-<yardstick>-<mode>-<utc time>/.
# Stops the whole run at RIG_MAX_MINUTES (default 40).

set -uo pipefail

yard=${1:?usage: run.sh <tyl|35xp> <perstep|single> [scratch-dir]}
mode=${2:?usage: run.sh <tyl|35xp> <perstep|single> [scratch-dir]}
here=$(cd "$(dirname "$0")" && pwd -P)
rig="$here/../042-rig"
tree=$(git -C "$here" rev-parse --show-toplevel)
scratch=${3:-${RIG_SCRATCH:-/Users/nickmele/Library/Application Support/Armada/attachments/job-scratch/4e9d8cea-3d92-43de-b5cc-2cbb6d77659d/devfleet}}
max=${RIG_MAX_MINUTES:-40}
home="$scratch/home/user"
origin="$scratch/origin.git"
bugjson="$tree/.armada/workflows/bug.json"

case "$yard" in
  tyl) yardfile="$rig/yardstick.md"; testfile=src/renderer/src/mock/scenario.test.ts; vitest_args="$testfile" ;;
  35xp) yardfile="$here/yardstick-2.md"; testfile=src/renderer/src/mock/checks-keys.test.tsx; vitest_args="--project 'smoke (browser)' $testfile" ;;
  *) echo "run.sh: yardstick is tyl or 35xp" >&2; exit 2 ;;
esac

runtime="$home/Library/Application Support/Armada/fleet.json"
pid=$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["pid"])' "$runtime" 2>/dev/null || true)
if [ -z "$pid" ] || ! kill -0 "$pid" 2>/dev/null; then
  echo "run.sh: no Fleet is up for $scratch (start one with $rig/fleet.sh)" >&2
  exit 1
fi
port=$(python3 -I -c 'import json,sys; print(json.load(open(sys.argv[1]))["port"])' "$runtime")
api="http://127.0.0.1:$port"
tag="$yard-$mode-$(date -u +%Y%m%dT%H%M%SZ)"
out="$scratch/runs/perstep-$tag"
mkdir -p "$out/transcripts"
run_start=$(date +%s)
main_before=$(git --git-dir="$origin" rev-parse refs/heads/main)
branches_before=$(git --git-dir="$origin" for-each-ref --format='%(refname:short)' refs/heads | sort | tr '\n' ' ')

log() { printf '%s\t%s\n' "$(date -u +%Y-%m-%dT%H:%M:%S.000Z)" "$*" | tee -a "$out/events.log"; }
elapsed_min() { echo $(( ($(date +%s) - run_start) / 60 )); }
remaining_min() { python3 -I -c "import sys; print(max(0.2, $max - ($(date +%s) - $run_start) / 60))"; }

# The settings: workflows off, as the rig's baseline does (the file is read live).
python3 -I - "$home/Library/Application Support/Armada/settings.json" <<'PY'
import json, os, sys, time
path = sys.argv[1]
data = json.load(open(path)) if os.path.exists(path) else {}
if data.get("features.workflows") is not False:
    data["features.workflows"] = False
    json.dump(data, open(path, "w"), indent=2)
    time.sleep(5)
PY

start_job() { # title -> id
  body=$(python3 -I - "$1" <<'PY'
import json, sys
print(json.dumps({"manifest_id": "armada", "title": sys.argv[1], "model": "sonnet", "effort": "medium", "mode": "auto"}))
PY
)
  curl -sf -X POST "$api/sessions/start" -H 'content-type: application/json' -d "$body" | python3 -I -c 'import json,sys; print(json.load(sys.stdin)["id"])'
}

send_message() { # sid file
  python3 -I - "$1" "$2" "$api" <<'PY'
import json, sys, urllib.request
sid, path, api = sys.argv[1:]
req = urllib.request.Request(api + "/sessions/message", json.dumps({"session_id": sid, "text": open(path).read()}).encode(), {"content-type": "application/json"})
urllib.request.urlopen(req, timeout=60).read()
PY
}

keep_transcripts() { # sid dest
  mkdir -p "$2"
  find "$home/.claude/projects" -name "*$1*" -print0 | while IFS= read -r -d '' f; do
    cp -R "$f" "$2/$(echo "$f" | sed 's|.*/projects/||; s|/|__|g')"
  done
}

verify_at() { # branch label -> prints PASS or FAIL and the test lines
  local d="$scratch/verify-$tag-$2"
  git clone --quiet --local --branch "$1" "$origin" "$d" 2>&1 || { echo "FAIL no such branch on origin: $1"; return; }
  (cd "$d" && bash "$rig/seed-node-modules.sh" "$tree" >/dev/null)
  local res
  res=$(cd "$d/apps/desktop" && eval "node_modules/.bin/vitest run $vitest_args" 2>&1 | grep -E "Tests |Test Files|FAIL|Error:" | head -8)
  if echo "$res" | grep -q "Tests .*passed" && ! echo "$res" | grep -q "failed"; then echo "PASS"; else echo "FAIL"; fi
  echo "$res"
}

# run_stretch <n> <message-file> <title> [person.py extra args...]  -> sets SID, REASON
run_stretch() {
  local n=$1 msg=$2 title=$3
  shift 3
  SID=$(start_job "$title") || { log "stretch $n: could not start a Job"; return 1; }
  echo "$SID" >"$out/session_id-$n-$SID"
  log "stretch $n: Job $SID started"
  send_message "$SID" "$msg"
  log "stretch $n: first message sent ($(wc -c <"$msg" | tr -d ' ') bytes)"
  REASON=$(python3 -I "$here/person.py" watch --scratch "$scratch" --session "$SID" --acts "$out/acts-$n-${SID:0:8}.jsonl" \
    --max-minutes "$(remaining_min)" --quiet-seconds 300 "$@")
  log "stretch $n: agent $SID stopped because: $REASON ($(elapsed_min) min into the run)"
  keep_transcripts "$SID" "$out/transcripts/$n-${SID:0:8}"
  curl -sf -X POST "$api/sessions/close" -H 'content-type: application/json' -d "{\"session_id\":\"$SID\"}" >/dev/null || true
  python3 -I "$rig/timeline.py" --scratch "$scratch" --session "$SID" --acts "$out/acts-$n-${SID:0:8}.jsonl" --out "$out/timeline-$n-${SID:0:8}.md" | tail -1 >"$out/summary-$n-${SID:0:8}.txt"
}

restore_main() {
  now=$(git --git-dir="$origin" rev-parse refs/heads/main)
  if [ "$now" != "$main_before" ]; then
    # compare-and-swap: only back to where it was if nobody else moved it since
    if [ -n "${merged_sha:-}" ] && git --git-dir="$origin" update-ref refs/heads/main "$main_before" "$merged_sha"; then
      log "origin main put back to $main_before (it was $merged_sha after the person's merge)"
    else
      log "origin main is $now, not put back (not the merge this run made)"
    fi
  fi
}

# ---------------------------------------------------------------- single
if [ "$mode" = single ]; then
  python3 -I - "$yardfile" "$out/brief.txt" <<'PY'
import sys
t = open(sys.argv[1]).read()
s = t.index("## Brief") + len("## Brief")
open(sys.argv[2], "w").write(t[s: t.index("## What the right answer looks like")].strip() + "\n")
PY
  run_stretch 1 "$out/brief.txt" "yardstick $yard single" || exit 1
  b="armada/session-${SID:0:8}"
  log "single: verify $b"
  verify_at "$b" 1 | tee "$out/verdict.txt"
  log "single: done in $(elapsed_min) min"
  exit 0
fi

# ---------------------------------------------------------------- perstep
work_branch=""
handoff=""
retry_note=""
gate_ok=1
for n in 1 2 3; do
  tries=0
  while :; do
    tries=$((tries + 1))
    msg="$out/message-$n-try$tries.txt"
    args=(--yardstick "$yardfile" --bug-json "$bugjson" --stretch "$n")
    [ -n "$work_branch" ] && args+=(--branch "$work_branch")
    [ -n "$handoff" ] && args+=(--handoff "$handoff")
    [ -n "$retry_note" ] && args+=(--retry-note "$retry_note")
    python3 -I "$here/compose.py" "${args[@]}" >"$msg"
    extra=()
    if [ "$n" = 3 ]; then
      [ -n "$work_branch" ] || { log "stretch 3: no working branch to deliver"; gate_ok=0; break 2; }
      extra=(--merge-branch "$work_branch" --origin "$origin" --merge-workdir "$scratch/merge-$tag")
    fi
    run_stretch "$n" "$msg" "perstep $yard stretch $n" ${extra[@]+"${extra[@]}"} || { gate_ok=0; break 2; }
    prev_sid=$SID
    retry_note=""
    if [ "$REASON" != done ]; then
      log "stretch $n: ended without done ($REASON); the run stops"
      gate_ok=0
      break 2
    fi

    # ---- the gate that ends the stretch
    case "$n" in
      1)
        if grep -q '"approve_plan"' "$out/acts-1-${SID:0:8}.jsonl" 2>/dev/null; then
          log "gate 1 closed: the person approved the plan at $(grep '"approve_plan"' "$out/acts-1-${SID:0:8}.jsonl" | head -1 | python3 -I -c 'import json,sys; print(json.loads(sys.stdin.readline())["at"])')"
          cand="armada/session-${SID:0:8}"
          if git -C "$scratch/repo" show-ref --verify --quiet "refs/heads/$cand"; then work_branch=$cand; log "stretch 1 left the branch $cand (no commits)"; else log "stretch 1 left no branch"; fi
        else
          log "gate 1 did not close: no plan approved"; gate_ok=0; break 2
        fi
        ;;
      2)
        if [ -z "$work_branch" ]; then work_branch="armada/session-${SID:0:8}"; fi
        if ! git --git-dir="$origin" rev-parse --verify --quiet "refs/heads/$work_branch" >/dev/null; then
          new=$(comm -13 <(echo "$branches_before" | tr ' ' '\n' | sort) <(git --git-dir="$origin" for-each-ref --format='%(refname:short)' refs/heads | sort) | tr '\n' ' ')
          log "stretch 2: $work_branch is not on origin; branches new since the run began: ${new:-none}"
          if [ "$(echo $new | wc -w | tr -d ' ')" = 1 ]; then work_branch=$(echo $new); log "stretch 2: taking $work_branch as the branch"; fi
        fi
        res=$(verify_at "$work_branch" "g2-$tries")
        echo "$res" >"$out/gate2-try$tries.txt"
        if [ "$(echo "$res" | head -1)" = PASS ]; then
          log "gate 2 closed: $testfile passes at $work_branch"
        else
          log "gate 2 did not close (try $tries): $(echo "$res" | sed -n 2,3p | tr '\n' ' ')"
          if [ "$tries" -lt 2 ] && [ "$(remaining_min | cut -d. -f1)" -ge 3 ]; then
            printf 'The runner ran %s at the commit pushed to %s and it did not pass:\n%s\nThe stretch is not over; fix it, push, and write your handoff again.\n' "$testfile" "$work_branch" "$res" >"$out/retry-note-$tries.txt"
            retry_note="$out/retry-note-$tries.txt"
            python3 -I "$here/transcripts.py" handoff --home "$home" --session "$SID" >"$out/handoff-2-try$tries.md"
            handoff="$out/handoff-2-try$tries.md"
            log "stretch 2: a second agent is started on the failure"
            continue
          fi
          gate_ok=0; break 2
        fi
        ;;
      3)
        mb=$(git --git-dir="$origin" rev-parse refs/heads/main)
        if git --git-dir="$origin" merge-base --is-ancestor "refs/heads/$work_branch" refs/heads/main 2>/dev/null && [ "$mb" != "$main_before" ]; then
          merged_sha=$mb
          log "gate 3 closed: origin main is $mb, holding $work_branch"
        else
          log "gate 3 did not close: origin main does not hold $work_branch"; gate_ok=0
        fi
        ;;
    esac

    # ---- the handoff the next agent starts from
    if [ "$n" -lt 3 ]; then
      h="$out/handoff-$n.md"
      python3 -I "$here/transcripts.py" handoff --home "$home" --session "$prev_sid" >"$h"
      if [ ! -s "$h" ]; then
        log "stretch $n: the agent wrote no HANDOFF message"
        echo "(the previous agent wrote no handoff)" >"$h"
      else
        log "stretch $n: handoff of $(wc -w <"$h" | tr -d ' ') words"
      fi
      handoff=$h
    fi
    break
  done
done

restore_main
log "run over: gate_ok=$gate_ok, $(elapsed_min) min; work branch ${work_branch:-none}"
echo "$work_branch" >"$out/work_branch"
echo "run.sh: output $out"
