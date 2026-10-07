import { expect, test } from 'claude-code/testing'

import {
  artifactOf,
  ghAct,
  isDispatch,
  jobIdsIn,
  mayMoveBranch,
  needAct,
  micros,
  pullRequestsIn,
  senderOf,
  customTitleIn,
  titleOf,
  transcriptPath,
} from './facts'

test('a title is the first line of a prompt, cut, and never a command', () => {
  expect(titleOf('\n  fix the   ledger intake\nand its tests')).toBe('fix the ledger intake')
  expect(titleOf('/compact')).toBeUndefined()
  expect(titleOf('   \n  ')).toBeUndefined()
  expect(titleOf('x'.repeat(200))?.length).toBe(80)
})

test('a git command that may move the branch is told from one that cannot', () => {
  expect(mayMoveBranch('git checkout -b fleet/a')).toBe(true)
  expect(mayMoveBranch('cd x && git switch main')).toBe(true)
  expect(mayMoveBranch('git -C /tmp/x worktree add y')).toBe(true)
  expect(mayMoveBranch('git status')).toBe(false)
  expect(mayMoveBranch('git log --oneline')).toBe(false)
})

test('a pull request is read off its address, once', () => {
  const text = 'https://github.com/Mele-Labs/armada/pull/1853\nsee also https://github.com/Mele-Labs/armada/pull/1853 and https://github.com/o/r/pull/7'
  expect(pullRequestsIn(text).map(one => one.number)).toEqual(['1853', '7'])
  expect(pullRequestsIn('no address here')).toEqual([])
})

test('a gh command is read for the act and the number it names', () => {
  expect(ghAct('gh pr create --base main')).toEqual({ act: 'create', number: undefined })
  expect(ghAct('gh pr merge 12 --merge')).toEqual({ act: 'merge', number: '12' })
  expect(ghAct('gh pr merge #12')).toEqual({ act: 'merge', number: '12' })
  expect(ghAct('gh pr close')).toEqual({ act: 'close', number: undefined })
  expect(ghAct('gh pr view 12')).toBeUndefined()
})

test('a dispatch is an armada-fleet tool that makes or sends a Job', () => {
  expect(isDispatch('mcp__armada-fleet__propose_job')).toBe(true)
  expect(isDispatch('mcp__armada-fleet__list_jobs')).toBe(false)
  expect(isDispatch('mcp__other__propose_job')).toBe(false)
})

test('Job ids are ULIDs in an answer, each once and at most ten', () => {
  const one = '01J8ZQ4W8N5T6V7Y9B2C3D4E5F'
  expect(jobIdsIn(`created ${one} and ${one}`)).toEqual([one])
  expect(jobIdsIn('nothing')).toEqual([])
  const many = Array.from({ length: 12 }, (_, n) => `01J8ZQ4W8N5T6V7Y9B2C3D4E${String(10 + n)}`)
  expect(jobIdsIn(many.join(' ')).length).toBeLessThanOrEqual(10)
})

test('a sender is the teammate where there is one and the kind where there is not', () => {
  expect(senderOf({ kind: 'peer', teammate: 'researcher' })).toBe('researcher')
  expect(senderOf({ kind: 'peer' })).toBe('peer')
})

test('cost goes out in millionths of a dollar', () => {
  expect(micros(1.25)).toBe(1_250_000)
  expect(micros(undefined)).toBeUndefined()
})

test('an armada need is read in its three forms and in no other', () => {
  expect(needAct('armada need ./a.toml "a minor"')).toEqual({ act: 'declare', path: 'a.toml', what: 'a minor' })
  expect(needAct("cd x && armada need a.toml 'two words'")).toEqual({
    act: 'declare',
    path: 'a.toml',
    what: 'two words',
  })
  expect(needAct('armada need --took a.toml "23.5"')).toEqual({ act: 'took', path: 'a.toml', value: '23.5' })
  expect(needAct('armada need --release a.toml')).toEqual({ act: 'release', path: 'a.toml' })
  expect(needAct('armada need --status')).toBeUndefined()
  expect(needAct('armada need a.toml')).toBeUndefined()
  expect(needAct('echo armada need a.toml "x"')).toBeUndefined()
  expect(needAct('armada land')).toBeUndefined()
})

test('a title never shows the markup a harness wrapped a prompt in', () => {
  expect(titleOf('<agent-message from="a45d14071172cd311">\nreview the ledger change\nthanks</agent-message>')).toBe(
    'review the ledger change',
  )
  expect(titleOf('<system-reminder>\nnote this\n</system-reminder>\nfix the intake')).toBe('fix the intake')
  expect(titleOf('<command-name>/compact</command-name>\n<command-args></command-args>')).toBeUndefined()
  expect(titleOf('<local-command-stdout>done</local-command-stdout>')).toBeUndefined()
  expect(titleOf('<agent-message from="a45d">\n</agent-message>')).toBeUndefined()
  expect(titleOf('make the <div> wrap')).toBe('make the <div> wrap')
})

test('a rename is read off the last custom-title entry of a transcript', () => {
  const lines = [
    '{"type":"user","message":"hi"}',
    '{"type":"custom-title","customTitle":"first name","sessionId":"S1"}',
    '{"type":"custom-title","customTitle":"  second   name ","sessionId":"S1"}',
    '{"type":"custom-title","customTi',
  ].join('\n')
  expect(customTitleIn(lines)).toBe('second name')
  expect(customTitleIn('{"type":"user"}')).toBeUndefined()
  expect(transcriptPath('/home/user', '/repos/armada/.armada', 'S1')).toBe(
    '/home/user/.claude/projects/-repos-armada--armada/S1.jsonl',
  )
})

test('a published page is an artifact at the address it answered with, and a read of one is not', () => {
  const url = 'https://claude.ai/artifact/abc123'
  expect(artifactOf('Artifact', { file_path: '/w/spike-notes.html' }, `Published ${url}`, false)).toEqual({
    form: 'page',
    target: url,
    title: 'spike-notes',
  })
  expect(artifactOf('Artifact', { url, file_path: '/w/a.html', title: 'Spike' }, 'ok', false)?.title).toBe('Spike')
  expect(artifactOf('Artifact', { action: 'list' }, url, false)).toBeUndefined()
  expect(artifactOf('Artifact', { action: 'read', url }, '<html>', false)).toBeUndefined()
  expect(artifactOf('Artifact', { url, asset: true, file_path: '/w/a.png' }, 'ok', false)).toBeUndefined()
  expect(artifactOf('Artifact', { file_path: '/w/a.html' }, 'refused', false)).toBeUndefined()
})

test('a new document, picture or pdf is an artifact, and code, an edit and scratch are not', () => {
  const made = (path: string, created = true) => artifactOf('Write', { file_path: path }, 'ok', created)
  expect(made('/repo/docs/spikes/clock.md')).toEqual({ form: 'file', target: '/repo/docs/spikes/clock.md', title: 'clock.md' })
  expect(made('/repo/shots/ledger.PNG')?.form).toBe('file')
  expect(made('/repo/out/report.pdf')?.form).toBe('file')
  expect(made('/repo/docs/clock.md', false)).toBeUndefined()
  expect(made('/repo/src/clock.ts')).toBeUndefined()
  expect(made('/repo/page.html')).toBeUndefined()
  expect(made('/repo/package.json')).toBeUndefined()
  expect(made('/tmp/notes.md')).toBeUndefined()
  expect(made('/repo/.claude/memory/notes.md')).toBeUndefined()
  expect(made('/repo/node_modules/x/README.md')).toBeUndefined()
  expect(artifactOf('Edit', { file_path: '/repo/docs/clock.md' }, 'ok', false)).toBeUndefined()
})

test('a Claude Docs document made or edited is an artifact, and reading one is not', () => {
  const url = 'https://claude.ai/artifact/doc9'
  const made = { container: { kind: 'project', create: { name: 'Spike write-up' } } }
  expect(artifactOf('mcp__claude_ai_Claude_Docs__batch', made, `Created ${url}`, false)).toEqual({
    form: 'doc',
    target: url,
    title: 'Spike write-up',
  })
  const edited = { ref: { object: 'node', id: 'n1' }, container: { kind: 'doc', id: 'doc9' }, payload: 'x' }
  expect(artifactOf('mcp__claude_ai_Claude_Docs__update', edited, 'ok', false)).toEqual({
    form: 'doc',
    target: url,
    title: undefined,
  })
  expect(artifactOf('mcp__claude_ai_Claude_Docs__read', edited, 'ok', false)).toBeUndefined()
  expect(artifactOf('mcp__claude_ai_Claude_Docs__guide', {}, 'ok', false)).toBeUndefined()
  expect(artifactOf('mcp__claude_ai_Claude_Docs__create', {}, 'no link', false)).toBeUndefined()
})
