#!/bin/bash
# usage: run.sh <label> <extra claude args...>
P=/private/tmp/claude-501/-Users-nickmele-Development-armada/d42cb58b-6440-4cec-8ce4-49863613ae18/scratchpad/probe
label=$1; shift
rm -f $P/marker-*.txt
cd $P/cwd
echo '{"type":"user","message":{"role":"user","content":"say ok"}}' | perl -e 'alarm 60; exec @ARGV' env -i PATH="$PATH" HOME=$P/fakehome LANG=en_US.UTF-8 TERM=dumb USER=$(id -un) \
  claude -p --input-format stream-json --output-format stream-json --verbose --replay-user-messages \
  --permission-mode default --strict-mcp-config --allowedTools "" "$@" > $P/out-$label.txt 2> $P/err-$label.txt
echo "[$label] exit=$? markers: $(ls $P/marker-*.txt 2>/dev/null | wc -l)"
tail -c 300 $P/out-$label.txt | head -c 300; echo
