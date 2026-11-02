import { expect, test } from 'claude-code/testing'

import { isOneArmadaJob, waitingItemsOfJob } from './jobcommand'

// The same lines `crates/fleet/src/tests/job_command.rs` holds, so the two readings do not drift.
test('one armada job command is allowed whatever it is given', () => {
  for (const line of [
    'armada job',
    'armada job status',
    'armada job finish_up',
    `armada job dispatch repo=web title="Fix the login" message='a; b | c && d'`,
    `armada job waiting_for '{"items":[{"id":"a","text":"Restart; now"}]}'`,
    `armada job record_plan "two words" '$HOME \`x\`'`,
    'armada   job\tstatus ',
    "armada job message='two\nlines in quotes'",
    "armada job record_plan - <<'PLAN'\n{\"summary\":\"a; b `c` $d\"}\nPLAN",
    "armada job record_plan - <<'PLAN'\n{}\nPLAN\n",
    "armada job waiting_for - <<'EOF'   \n{}\nEOF\n\n",
  ]) expect(isOneArmadaJob(line)).toBe(true)
})

test('a second command, a pipe, a redirect or an expansion is the shell and asks', () => {
  for (const line of [
    'armada job x; rm -rf ~',
    'armada job x && y',
    'armada job x || y',
    'armada job x | y',
    'armada job x & ',
    'armada job x > out',
    'armada job x < in',
    'armada job x 2>&1',
    'armada job x\nrm -rf ~',
    'armada job x\r',
    '(armada job x)',
    'armada job $HOME',
    'armada job ${X}',
    'armada job `y`',
    'armada job "$(y)"',
    'armada job "`y`"',
    'armada job "$HOME"',
    'armada job "a\\"b"',
    'armada job a\\ b',
    'armada job x "unterminated',
    "armada job x 'unterminated",
  ]) expect(isOneArmadaJob(line)).toBe(false)
})

test('only the program armada with the verb job is the shortcut', () => {
  for (const line of ['', 'armada', 'armada jobs', 'armada mcp', 'armada check test', 'rm armada job', 'FOO=1 armada job', '/usr/local/bin/armada job', 'echo armada job', 'armadajob'])
    expect(isOneArmadaJob(line)).toBe(false)
  expect(isOneArmadaJob(undefined)).toBe(false)
})

test('a heredoc that could expand, run on or leave stdin unnamed is refused', () => {
  for (const line of [
    'armada job x - <<EOF\n{}\nEOF',
    'armada job x - <<"EOF"\n{}\nEOF',
    "armada job x - <<-'EOF'\n{}\nEOF",
    "armada job x <<'EOF'\n{}\nEOF",
    "armada job x - <<'EOF'\n{}",
    "armada job x - <<'EOF'\n{}\nEOF\nrm -rf ~",
    "armada job x - <<'EOF' | sh\n{}\nEOF",
    "armada job x - <<'EOF'; rm -rf ~\n{}\nEOF",
    "armada job x - <<'EOF' && y\n{}\nEOF",
    "armada job x - <<''\n{}\n",
    "armada job x - <<'EOF'\n{}\nEOF extra",
    "rm - <<'EOF'\n{}\nEOF",
  ]) expect(isOneArmadaJob(line)).toBe(false)
})

test('the list a waiting_for command sets is read from its argument, its heredoc or its items field', () => {
  const list = [{ id: 'a', text: 'Restart' }]
  expect(waitingItemsOfJob(`armada job waiting_for '{"items":[{"id":"a","text":"Restart"}]}'`)).toEqual(list)
  expect(waitingItemsOfJob(`armada job waiting-for '{"items":[{"id":"a","text":"Restart"}]}'`)).toEqual(list)
  expect(waitingItemsOfJob(`armada job waiting_for items='[{"id":"a","text":"Restart"}]'`)).toEqual(list)
  expect(waitingItemsOfJob(`armada job waiting_for - <<'T'\n{"items":[{"id":"a","text":"Restart"}]}\nT`)).toEqual(list)
  expect(waitingItemsOfJob(`armada job waiting_for '{"items":[]}'`)).toEqual([])
})

test('a line that is not a waiting_for command, or names no list, sets none', () => {
  expect(waitingItemsOfJob('armada job finish_up')).toBeUndefined()
  expect(waitingItemsOfJob('armada job waiting_for')).toBeUndefined()
  expect(waitingItemsOfJob(`armada job waiting_for '{"note":1}'`)).toBeUndefined()
  expect(waitingItemsOfJob(`armada job waiting_for '{"items":[]}'; rm x`)).toBeUndefined()
  expect(waitingItemsOfJob(`echo armada job waiting_for '{"items":[]}'`)).toBeUndefined()
})
