#!/bin/sh
# ARMADA_AGENT_BINARY: Fleet runs this as the hosted Job's agent (the rig's bin/agent, plus one branch).
#
# Spike 46. The mod a hosted Job loads is the folder Fleet passes as `--plugin-dir`. Spike 46 needs its own
# copy of the mod (variant-046/mod/armada) for ONE Job on a Fleet that other spikes share, so this wrapper
# swaps that argument for exactly one session:
#   variant-046/claim     exists only between run.sh arming it and the Job's first launch
#   variant-046/session   the id of the first FRESH launch (`--session-id`, never a `--resume`) while the claim stood; every later launch of
#                         that id (the restart into a slot is `--resume <id>`) gets the copy, no other does
# With no claim file, or any other session, this is the rig's wrapper unchanged. Every launch is logged.
here=$(cd "$(dirname "$0")" && pwd -P)
v="$here/../variant-046"
mine="$v/mod/armada"

id=""
fresh=0
prev=""
for a in "$@"; do
  case "$prev" in
    --session-id) id=$a; fresh=1 ;;
    --resume) id=$a ;;
  esac
  prev=$a
done

use=0
if [ -f "$v/session" ]; then
  [ "$id" = "$(cat "$v/session")" ] && use=1
elif [ -f "$v/claim" ] && [ "$fresh" = 1 ]; then
  printf '%s' "$id" >"$v/session"
  use=1
fi
[ -d "$v" ] && printf '%s launch %s variant_mod=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$id" "$use" >>"$v/wrapper.log"

if [ "$use" = 1 ]; then
  n=$#
  swap=0
  while [ "$n" -gt 0 ]; do
    a=$1
    shift
    if [ "$swap" = 1 ]; then
      a=$mine
      swap=0
    elif [ "$a" = "--plugin-dir" ]; then
      swap=1
    fi
    set -- "$@" "$a"
    n=$((n - 1))
  done
fi
exec "${RIG_CLAUDE:-/Users/nickmele/.local/bin/claude}" "$@"
