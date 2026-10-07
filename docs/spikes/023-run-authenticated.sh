#!/bin/bash
# One authenticated turn with the Drone's environment and the real HOME, to see the flag leaves auth alone.
P=/private/tmp/claude-501/-Users-nickmele-Development-armada/d42cb58b-6440-4cec-8ce4-49863613ae18/scratchpad/probe
cd $P/cwd
echo '{"type":"user","message":{"role":"user","content":"Reply with the single word ok."}}' | perl -e 'alarm 90; exec @ARGV' env -i PATH="$PATH" HOME=$HOME LANG=en_US.UTF-8 TERM=dumb USER=$(id -un) \
  claude -p --input-format stream-json --output-format stream-json --verbose --model haiku --max-turns 1 \
  --permission-mode default --strict-mcp-config --allowedTools "" --no-session-persistence "$@" > $P/out-real.txt 2> $P/err-real.txt
echo "exit=$?"
python3 -I -c "
import json
for l in open('$P/out-real.txt'):
    j=json.loads(l)
    if j.get('type')=='result': print(j.get('is_error'), j.get('result'))
    if j.get('type')=='system' and j.get('subtype')=='init': print('plugins', j.get('plugins'), 'hooks?', [k for k in j if 'hook' in k.lower()], 'tools', len(j.get('tools',[])))
"
