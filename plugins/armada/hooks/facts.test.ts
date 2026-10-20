import { expect, test } from 'claude-code/testing'

import {
  promptDetail,
  artifactOf,
  backgroundEndOf,
  backgroundOf,
  commitIn,
  effortOf,
  ghAct,
  ghIssueCreate,
  issuesIn,
  modeOf,
  modelName,
  madeTask,
  mayMoveBranch,
  micros,
  planOf,
  pullRequestsIn,
  pushIn,
  senderOf,
  spentUntil,
  customTitleIn,
  titleOf,
  todoTasks,
  saidOf,
  touchedOf,
  transcriptPath,
  updatedTask,
  usedFact,
  agentItemsIn,
  isCovered,
  needsSaidIn,
  sentBackReason,
  waitingItemsOf,
  withReplyNeeds,
  agentOrigin,
  agentUseOrigin,
  loadoutOf,
  memoryItem,
  searchedOf,
  serverItem,
  serverOfTool,
  skillOrigin,
  skillUseOrigin,
  styleOrigin,
  toolOrigin,
  waitedOf,
} from './facts'
import { BASH_LAUNCH, NOTICE_COMPLETED, NOTICE_KILLED } from './background.fixture'

test('a title is the first line of a prompt, cut, and never a command', () => {
  expect(titleOf('\n  fix the   ledger intake\nand its tests')).toBe('fix the ledger intake')
  expect(titleOf('/compact')).toBeUndefined()
  expect(titleOf('   \n  ')).toBeUndefined()
  expect(titleOf('x'.repeat(200))?.length).toBe(80)
})

test('what a turn said is the first line of its answer, flat and cut, and nothing for a turn that said nothing', () => {
  expect(saidOf('\n  The cap   is five.\nAnd the backoff stops there.')).toBe('The cap is five.')
  expect(saidOf('x'.repeat(300))?.length).toBe(200)
  expect(saidOf('  \n ')).toBeUndefined()
  expect(saidOf('')).toBeUndefined()
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
  // Auto-merge only asks: nothing has merged yet.
  expect(ghAct('gh pr merge 2027 --merge --auto')).toBeUndefined()
  expect(ghAct('gh pr merge --auto --merge 2027 | tail -1')).toBeUndefined()
  expect(ghAct('gh pr close')).toEqual({ act: 'close', number: undefined })
  expect(ghAct('gh pr view 12')).toBeUndefined()
})

test('an issue is read off its address, once', () => {
  const text = 'https://github.com/Mele-Labs/armada/issues/2214\nhttps://github.com/Mele-Labs/armada/issues/2214 and https://github.com/Mele-Labs/armada/pull/7'
  expect(issuesIn(text)).toEqual([{ number: '2214', url: 'https://github.com/Mele-Labs/armada/issues/2214' }])
  expect(issuesIn('#2214 is no address')).toEqual([])
})

test('a gh command that files an issue is told from one that reads one', () => {
  expect(ghIssueCreate('gh issue create --title x --body-file b.md')).toBe(true)
  expect(ghIssueCreate('cd /repo && gh issue create -t x | tail -1')).toBe(true)
  expect(ghIssueCreate('gh issue view 2214')).toBe(false)
  expect(ghIssueCreate('gh pr create --fill')).toBe(false)
})

test('a sender is the teammate where there is one and the kind where there is not', () => {
  expect(senderOf({ kind: 'peer', teammate: 'researcher' })).toBe('researcher')
  expect(senderOf({ kind: 'peer' })).toBe('peer')
})

test('cost goes out in millionths of a dollar', () => {
  expect(micros(1.25)).toBe(1_250_000)
  expect(micros(undefined)).toBeUndefined()
})

test('a model is named by its short name, a mode in Armada words and an effort whichever way it came', () => {
  expect(modelName('claude-sonnet-5-5')).toBe('sonnet')
  expect(modelName('some-other-model')).toBe('some-other-model')
  expect(modeOf('acceptEdits')).toBe('accept_edits')
  expect(modeOf('default')).toBe('ask')
  expect(modeOf('bypassPermissions')).toBeUndefined()
  expect(effortOf('low')).toBe('low')
  expect(effortOf({ level: 'high' })).toBe('high')
  expect(effortOf(undefined)).toBeUndefined()
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

test('a picture a session reads is an image artifact, and code and a written picture are not', () => {
  const looked = (path: string) => artifactOf('Read', { file_path: path }, 'ok', false)
  expect(looked('/tmp/shot.PNG')).toEqual({ form: 'image', target: '/tmp/shot.PNG', title: 'shot.PNG' })
  expect(looked('/repo/a.jpeg')?.form).toBe('image')
  expect(looked('/repo/src/clock.ts')).toBeUndefined()
  expect(looked('/repo/docs/clock.md')).toBeUndefined()
  expect(artifactOf('Write', { file_path: '/repo/shot.png' }, 'ok', true)?.form).toBe('file')
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

test('a plan window is told in whole percent with its reset as an ISO instant', () => {
  expect(
    planOf([
      { kind: 'five_hour', percentUsed: 41.6, resetsAt: '2026-10-12T22:40:00Z' },
      { kind: 'seven_day', percentUsed: 7 },
      { kind: 'spend_limit', percentUsed: 12, resetsAt: 'tomorrow-ish' },
    ]),
  ).toEqual([
    { kind: 'five_hour', percent_used: 42, resets_at: '2026-10-12T22:40:00.000Z' },
    { kind: 'seven_day', percent_used: 7 },
    { kind: 'spend_limit', percent_used: 12 },
  ])
  expect(planOf([])).toEqual([])
})

test('a spent limit opens again at the latest reset among the windows at or near full', () => {
  const five = { kind: 'five_hour', percentUsed: 100, resetsAt: '2026-10-12T22:40:00Z' }
  const week = { kind: 'seven_day', percentUsed: 99.4, resetsAt: '2026-10-15T09:00:00Z' }
  expect(spentUntil([five])).toBe('2026-10-12T22:40:00.000Z')
  expect(spentUntil([five, week])).toBe('2026-10-15T09:00:00.000Z')
  expect(spentUntil([five, { ...week, percentUsed: 60 }])).toBe('2026-10-12T22:40:00.000Z')
})

test('nothing is said of a limit no window shows full, or whose reset is unknown', () => {
  expect(spentUntil([{ kind: 'five_hour', percentUsed: 80, resetsAt: '2026-10-12T22:40:00Z' }])).toBeUndefined()
  expect(spentUntil([{ kind: 'five_hour', percentUsed: 100 }])).toBeUndefined()
  expect(spentUntil([])).toBeUndefined()
})

test('a written file is named from the session directory, and one outside it is not told', () => {
  expect(touchedOf('/repos/armada', { file_path: '/repos/armada/src/a.rs' })).toBe('src/a.rs')
  expect(touchedOf('/repos/armada/', { file_path: './src/a.rs' })).toBe('src/a.rs')
  expect(touchedOf('/repos/armada', { notebook_path: '/repos/armada/n.ipynb' })).toBe('n.ipynb')
  expect(touchedOf('/repos/armada', { file_path: '/repos/armada-other/a.rs' })).toBeUndefined()
  expect(touchedOf('/repos/armada', { file_path: '/etc/hosts' })).toBeUndefined()
  expect(touchedOf('/repos/armada', { file_path: '../x/a.rs' })).toBeUndefined()
  expect(touchedOf('/repos/armada', { content: 'x' })).toBeUndefined()
})

test('a read is told as a path, a command as its first line, and a write as before', () => {
  expect(usedFact('read', '/repos/armada', { file_path: '/repos/armada/src/a.rs' })).toEqual({ kind: 'read', path: 'src/a.rs' })
  expect(usedFact('read', '/repos/armada', { file_path: '/etc/hosts' })).toBeUndefined()
  expect(usedFact('touched', '/repos/armada', { file_path: '/repos/armada/src/a.rs' })).toEqual({ kind: 'touched', paths: ['src/a.rs'] })
  expect(usedFact('touched', '/repos/armada', { file_path: '/repos/armada/src/a.rs' }, { how: 'write', agent: 'a1' })).toEqual({
    kind: 'touched',
    paths: ['src/a.rs'],
    act: 'write',
    by: 'a1',
  })
  expect(usedFact('ran', '/repos/armada', { command: '\n cargo test\nand more' })).toEqual({ kind: 'ran', command: 'cargo test' })
  expect(usedFact('ran', '/repos/armada', { command: 'x'.repeat(500) })).toEqual({ kind: 'ran', command: 'x'.repeat(300) })
  expect(usedFact('ran', '/repos/armada', { command: '  ' })).toBeUndefined()
  expect(usedFact('ran', '/repos/armada', {})).toBeUndefined()
})

test('a TodoWrite list is told whole, each item with its place as the id, and an empty one clears it', () => {
  expect(
    todoTasks({
      todos: [
        { content: 'Read the intake', status: 'completed', activeForm: 'Reading' },
        { content: '  Write the   migration ', status: 'in_progress' },
        { content: '', status: 'pending' },
        { content: 'Test it', status: 'later' },
      ],
    }),
  ).toEqual([
    { id: '1', text: 'Read the intake', status: 'completed' },
    { id: '2', text: 'Write the migration', status: 'in_progress' },
    { id: '3', text: 'Test it', status: 'pending' },
  ])
  expect(todoTasks({ todos: [] })).toEqual([])
  expect(todoTasks({ file_path: 'a' })).toBeUndefined()
  expect(todoTasks({ todos: [{ content: 'x'.repeat(300), status: 'pending' }] })?.[0].text.length).toBe(200)
})

test('a task made takes the id the tool gave it, from its result or its words, else the next place', () => {
  const none = madeTask([], { subject: 'Read the intake' }, { task: { id: '7' } }, '')
  expect(none).toEqual([{ id: '7', text: 'Read the intake', status: 'pending' }])
  expect(madeTask(none!, { subject: 'Test it' }, {}, 'Task #8 created successfully: Test it')?.map(one => one.id)).toEqual(['7', '8'])
  expect(madeTask(none!, { subject: 'Ship it' }, undefined, 'created')?.map(one => one.id)).toEqual(['7', '2'])
  expect(madeTask([], { description: 'no subject' }, {}, '')).toBeUndefined()
  expect(madeTask(none!, { subject: 'Child' }, { task: { id: '8' } }, '', 'a1')?.[1]).toEqual({ id: '8', text: 'Child', status: 'pending', by: 'a1' })
})

test('a task updated changes its status or its words, goes when deleted, and is not invented when never seen', () => {
  const list = [
    { id: '1', text: 'Read the intake', status: 'pending' as const },
    { id: '2', text: 'Test it', status: 'pending' as const },
  ]
  expect(updatedTask(list, { taskId: '1', status: 'completed' })?.map(one => one.status)).toEqual(['completed', 'pending'])
  expect(updatedTask(list, { taskId: 2, subject: 'Test the intake' })?.[1].text).toBe('Test the intake')
  expect(updatedTask(list, { taskId: '1', status: 'deleted' })?.map(one => one.id)).toEqual(['2'])
  expect(updatedTask(list, { taskId: '1', status: 'pending' })).toBeUndefined()
  expect(updatedTask(list, { taskId: '9', status: 'completed' })).toBeUndefined()
})

test('a commit is read from what git printed, so one with nothing staged is not one', () => {
  expect(commitIn('git commit -m "Pin the clock"', '[armada/session-build-record 1a2b3c4] Pin the clock\n 2 files changed')).toEqual({
    branch: 'armada/session-build-record',
  })
  expect(commitIn('git add -A && git commit --amend --no-edit', '[main (root-commit) 1a2b3c4d5e] First')).toEqual({ branch: 'main' })
  expect(commitIn('git commit -m x', '[detached HEAD 1a2b3c4] x')).toEqual({})
  expect(commitIn('git commit -m x', 'nothing to commit, working tree clean')).toBeUndefined()
  expect(commitIn('git log -3', '[main 1a2b3c4] looks like one')).toBeUndefined()
})

test('a push is read from the ref line git printed, and a rejected, empty or dry one tells nothing', () => {
  const to = 'To https://github.com/Mele-Labs/armada.git\n'
  expect(pushIn('git push -u origin topic', to + ' * [new branch]      topic -> topic')).toEqual({ branch: 'topic' })
  expect(pushIn('git push', to + '   1a2b3c4..5d6e7f8  topic -> topic')).toEqual({ branch: 'topic' })
  expect(pushIn('git push --force-with-lease', to + ' + 1a2b3c4...5d6e7f8 topic -> topic (forced update)')).toEqual({ branch: 'topic' })
  expect(pushIn('git push', 'Everything up-to-date')).toBeUndefined()
  expect(pushIn('git push', to + ' ! [rejected]        topic -> topic (fetch first)')).toBeUndefined()
  expect(pushIn('git push --dry-run', to + '   1a2b3c4..5d6e7f8  topic -> topic')).toBeUndefined()
  expect(pushIn('git pull', to + '   1a2b3c4..5d6e7f8  topic -> topic')).toBeUndefined()
})

test('a permission prompt is named by its command line, its path or its address, never content', () => {
  expect(promptDetail({ command: 'cargo build\n--release' })).toBe('cargo build')
  expect(promptDetail({ file_path: '/etc/hosts', content: 'secret' })).toBe('/etc/hosts')
  expect(promptDetail({ url: 'https://example.com/a' })).toBe('https://example.com/a')
  expect(promptDetail({ command: 'x'.repeat(500) })).toBe('x'.repeat(300))
  expect(promptDetail({ prompt: 'what to fetch' })).toBe('')
})

// The owner's BLUF reply, as his output style writes it.
const BLUF = (rows: string) => `**BLUF:** The detector is built and its tests pass.

## Summary

The Stop hook now reads the reply.

## Actions

| # | Action | Owner | Status |
|---|---|---|---|
${rows}
`

test('a BLUF Actions table the agent owns names nothing the owner owes', () => {
  const reply = BLUF(
    [
      '| 1 | Wrote `needsSaidIn` in facts.ts | Agent | Done |',
      '| 2 | Ran the touched tests | Claude | Done |',
      '| 3 | Review the parser | Explore subagent | In progress |',
      '| 4 | Check the door names | rust-engineer | Not started |',
    ].join('\n'),
  )
  expect(needsSaidIn(reply)).toEqual([])
})

test('rows the owner owns and has not done are named, each as its action', () => {
  const reply = BLUF(
    [
      '| 1 | Opened PR #2415 | Agent | Done |',
      '| 2 | Approve PR #2415 | **Nick** | Not started |',
      '| 3 | Run `scripts/restart` after it merges | You | Not started |',
      '| 4 | Merged the rules branch | Nick | Done ✅ |',
      '| 5 | Decide whether the fallback posts to Fleet | Owner / Agent | Waiting |',
    ].join('\n'),
  )
  expect(needsSaidIn(reply)).toEqual([
    'Approve PR #2415',
    'Run scripts/restart after it merges',
    'Decide whether the fallback posts to Fleet',
  ])
})

test('a row the owner owns that is done names nothing', () => {
  expect(needsSaidIn(BLUF('| 1 | Approve PR #2410 | Nick | Done |'))).toEqual([])
  expect(needsSaidIn(BLUF('| 1 | Approve PR #2410 | Nick | ✅ Merged |'))).toEqual([])
})

test('a table with an Owner column counts under any heading, and one with none counts nowhere', () => {
  const owned = '## Leftovers\n\n| Item | Owner |\n|---|---|\n| Rotate the token | Nick |\n'
  expect(needsSaidIn(owned)).toEqual(['Rotate the token'])
  const ownerless = '## Actions\n\n| Step | Result |\n|---|---|\n| Ran tsc | clean |\n'
  expect(needsSaidIn(ownerless)).toEqual([])
  // Who is an owner column only in an Actions table.
  expect(needsSaidIn('## Results\n\n| What | Who |\n|---|---|\n| Restart Fleet | Nick |\n')).toEqual([])
  expect(needsSaidIn('## Next actions\n\n| What | Who |\n|---|---|\n| Restart Fleet | Nick |\n')).toEqual(['Restart Fleet'])
})

test('items under a Questions heading are named, and a section that asks nothing is not', () => {
  const reply = [
    'The route is in place.',
    '',
    '## Questions',
    '',
    '1. Should a reply line carry an act, or none?',
    '2. **Keep** the id prefix `reply:`?',
    '',
    '## Notes',
    '',
    '- The door names one server.',
  ].join('\n')
  expect(needsSaidIn(reply)).toEqual(['Should a reply line carry an act, or none?', 'Keep the id prefix reply:?'])
  expect(needsSaidIn('## Questions\n\nNone.\n')).toEqual([])
  expect(needsSaidIn('**Open questions:**\n- Which slot pool?\n')).toEqual(['Which slot pool?'])
})

test('a QUESTION: line is named wherever it stands', () => {
  expect(needsSaidIn('Built it.\n\nQUESTION: Bump to 0.3.26 or wait for #2414?')).toEqual([
    'Bump to 0.3.26 or wait for #2414?',
  ])
  // Lower case is prose, not the marker.
  expect(needsSaidIn('The question: does it loop? It does not.')).toEqual([])
})

test('a table or a question inside a fenced block is never read', () => {
  const reply = [
    'Here is what the owner style asks for:',
    '',
    '```markdown',
    '## Actions',
    '| # | Action | Owner | Status |',
    '|---|---|---|---|',
    '| 1 | Approve the PR | Nick | Not started |',
    'QUESTION: Which one?',
    '```',
    '',
    'Nothing is owed.',
  ].join('\n')
  expect(needsSaidIn(reply)).toEqual([])
})

test('the list a waiting_for call set is read from its body, and a shapeless one is not', () => {
  const items = [{ id: 'merge', text: 'Approve PR #2415', act: { kind: 'approve_pr', target: '#2415' } }]
  expect(waitingItemsOf({ body: { session_id: 'S1', waiting_for: items } })).toEqual(items)
  expect(waitingItemsOf({ body: JSON.stringify({ session_id: 'S1', waiting_for: [] }) })).toEqual([])
  expect(waitingItemsOf({ body: { session_id: 'S1' } })).toBeUndefined()
})

test('a list holds a line when an item shares two of its words', () => {
  const items = [{ id: 'merge', text: 'Approve #2415, the needs detector' }]
  expect(isCovered(['Approve PR #2415'], items)).toBe(true)
  expect(isCovered(['Run scripts/restart after it merges'], items)).toBe(false)
  expect(isCovered(['Approve PR #2415'], [])).toBe(false)
})

test('what the session is sent back with names each line, and the list adds only what it lacks', () => {
  expect(sentBackReason(['Approve PR #2415', 'Restart Fleet'], 'S1')).toBe(
    [
      'Your reply names what the owner owes, and the armada waiting_for list does not hold it:',
      '- Approve PR #2415',
      '- Restart Fleet',
      'Put each in waiting_for with its own id, one short line and an act, passing session_id S1, then end the turn. Do not repeat them as a table.',
    ].join('\n'),
  )
  const held = [{ id: 'merge', text: 'Approve PR #2415' }]
  expect(withReplyNeeds(held, ['Approve PR #2415', 'Restart Fleet'])).toEqual([
    { id: 'merge', text: 'Approve PR #2415' },
    { id: 'reply:1', text: 'From the reply: Restart Fleet' },
  ])
})

test("the agent's own items are read from the session Fleet holds, and the ones Fleet raised are left out", () => {
  const thread = JSON.stringify({
    session: {
      waiting_for: [
        { id: 'pick', text: 'Pick the slot pool', since: 'x', source: 'agent', steps: ['Open Kit'] },
        { id: 'ask:7', text: 'Which branch?', since: 'x', source: 'ask_card' },
        { id: 'perm:3', text: 'Allow cargo build', since: 'x', source: 'permission' },
      ],
    },
    rows: [],
  })
  expect(agentItemsIn(thread)).toEqual([{ id: 'pick', text: 'Pick the slot pool', steps: ['Open Kit'] }])
  expect(agentItemsIn(JSON.stringify({ session: { waiting_for: [] }, rows: [] }))).toEqual([])
  expect(agentItemsIn('{}')).toBeUndefined()
  expect(agentItemsIn('not json')).toBeUndefined()
})

test('a background command is the call with the id its result gave, and a call without one is not', () => {
  expect(backgroundOf('Bash', { command: 'pnpm dev\nmore', description: 'Serve it', run_in_background: true }, BASH_LAUNCH)).toEqual({
    target: 'b3wd556q5',
    detail: {
      form: 'command',
      task: 'Serve it',
      command: 'pnpm dev',
      output: '/private/tmp/claude-501/-Users-nickmele-Development-armada--armada-slots-slot-12/aa994f6f-56ce-4b09-9e06-66342d14548f/tasks/b3wd556q5.output',
    },
  })
  expect(backgroundOf('Bash', { command: 'pnpm dev' }, BASH_LAUNCH)).toBeUndefined()
  expect(backgroundOf('Bash', { command: 'pnpm dev', run_in_background: true }, 'refused')).toBeUndefined()
  // No Monitor result was recorded: the pattern is tolerant and this only holds the shapes it was written for.
  expect(backgroundOf('Monitor', { description: 'Watch', command: 'tail -f x', timeout_ms: 1 }, 'Monitor started (task mon42)')?.detail).toEqual({
    form: 'monitor',
    task: 'Watch',
    command: 'tail -f x',
  })
})

test('a task notice names the task, its word, the exit code the summary gives and the file it wrote', () => {
  expect(backgroundEndOf(NOTICE_COMPLETED)).toEqual({ target: 'byry3zett', status: 'completed', exit: '0', output: '/private/tmp/claude-501/.../tasks/byry3zett.output' })
  expect(backgroundEndOf(NOTICE_KILLED)).toEqual({ target: 'bdjviwn7c', status: 'killed', output: '/private/tmp/claude-501/.../tasks/bdjviwn7c.output' })
  expect(backgroundEndOf('Just a prompt')).toBeUndefined()
})

test('a skill is sourced by the engine\'s word for it, and one Armada has no word for has none', () => {
  expect(skillOrigin('userSettings', 'notes')).toEqual({ source: 'kit' })
  expect(skillOrigin('projectSettings', 'land')).toEqual({ source: 'repository' })
  expect(skillOrigin('plugin', 'armada:walk', 'armada')).toEqual({ source: 'armada' })
  expect(skillOrigin('plugin', 'productivity:start')).toEqual({ source: 'plugin', from: 'productivity' })
  expect(skillOrigin('built-in', 'loop')).toEqual({ source: 'built_in' })
  expect(skillOrigin('mcp', 'github:triage')).toEqual({ source: 'mcp', from: 'github' })
  expect(skillOrigin('policySettings', 'house')).toEqual({ source: 'managed' })
  expect(skillOrigin('syncedSkills', 'mine')).toEqual({ source: 'kit' })
  expect(skillOrigin('somewhereNew', 'x')).toBeUndefined()
})

test('a subagent is sourced by where it was defined, a plugin\'s by its prefix', () => {
  expect(agentOrigin('userSettings', 'scout')).toEqual({ source: 'kit' })
  expect(agentOrigin('localSettings', 'reviewer')).toEqual({ source: 'repository' })
  expect(agentOrigin('plugin', 'armada:judge')).toEqual({ source: 'armada' })
  expect(agentOrigin('plugin', 'productivity:planner')).toEqual({ source: 'plugin', from: 'productivity' })
  expect(agentOrigin('flagSettings', 'x')).toEqual({ source: 'managed' })
  expect(agentOrigin('policySettings', 'x')).toEqual({ source: 'managed' })
})

test('a memory file is the person\'s under ~/.claude, the repository\'s under its root, and a managed one an organisation\'s', () => {
  expect(memoryItem('/home/user/.claude/CLAUDE.md', 'User', '/home/user', '/repos/a')).toEqual({ name: '~/.claude/CLAUDE.md', source: 'kit' })
  expect(memoryItem('/repos/a/crates/CLAUDE.md', 'Project', '/home/user', '/repos/a/')).toEqual({ name: 'crates/CLAUDE.md', source: 'repository' })
  expect(memoryItem('/repos/CLAUDE.md', 'Project', '/home/user', '/repos/a')).toEqual({ name: '/repos/CLAUDE.md', source: 'repository' })
  expect(memoryItem('/etc/claude-code/CLAUDE.md', 'Managed', '/home/user', '/repos/a')).toEqual({ name: '/etc/claude-code/CLAUDE.md', source: 'managed' })
})

test('a tool is an MCP server\'s as the breakdown names it, else as its name spells it, else the harness\'s', () => {
  expect(serverOfTool('mcp__claude_ai_Gmail__create_draft')).toBe('claude_ai_Gmail')
  expect(serverOfTool('WebFetch')).toBeUndefined()
  expect(toolOrigin('mcp__claude_ai_Gmail__create_draft', [{ name: 'mcp__claude_ai_Gmail__create_draft', serverName: 'claude.ai Gmail' }])).toEqual({ source: 'mcp', from: 'claude.ai Gmail' })
  expect(toolOrigin('mcp__github__get_me')).toEqual({ source: 'mcp', from: 'github' })
  expect(toolOrigin('WebFetch')).toEqual({ source: 'built_in' })
})

test('an output style is Armada\'s, a plugin\'s, built in, the repository\'s or the person\'s', () => {
  expect(styleOrigin('Armada', false)).toEqual({ source: 'armada' })
  expect(styleOrigin('armada:Armada', false)).toEqual({ source: 'armada' })
  expect(styleOrigin('acme:Terse', false)).toEqual({ source: 'plugin', from: 'acme' })
  expect(styleOrigin('Explanatory', false)).toEqual({ source: 'built_in' })
  expect(styleOrigin('Terse', true)).toEqual({ source: 'repository' })
  expect(styleOrigin('Terse', false)).toEqual({ source: 'kit' })
})

test('a plugin\'s MCP server is that plugin\'s, Armada\'s is Armada\'s, and any other is its own', () => {
  expect(serverItem('plugin:productivity:asana')).toEqual({ name: 'asana', source: 'plugin', from: 'productivity' })
  expect(serverItem('plugin:armada:browser')).toEqual({ name: 'browser', source: 'armada', from: 'armada' })
  expect(serverItem('claude.ai Gmail')).toEqual({ name: 'claude.ai Gmail', source: 'mcp', from: 'claude.ai Gmail' })
})

test('a ToolSearch and a WaitForMcpServers result say which tools loaded and how each server stands, and no error text', () => {
  expect(searchedOf({ matches: ['WebFetch', 3], pending_mcp_servers: ['linear'], failed_mcp_servers: [{ name: 'slack', error: 'tunnel refused' }] })).toEqual({
    matches: ['WebFetch'],
    servers: { linear: 'pending', slack: 'failed' },
  })
  expect(searchedOf(undefined)).toEqual({ matches: [], servers: {} })
  expect(waitedOf({ connected: ['github'], failed: ['slack'], stillPending: ['linear'], needsAuth: ['notion'] })).toEqual({
    github: 'connected',
    slack: 'failed',
    linear: 'pending',
    notion: 'failed',
  })
})

test('the Loadout sources an organisation\'s items as managed, and a server with tools listed is connected whatever was said of it', () => {
  const items = loadoutOf({
    breakdown: {
      memoryFiles: [],
      mcpTools: [{ name: 'mcp__github__get_me', serverName: 'github', tokens: 0, isLoaded: true }],
      agents: [{ agentType: 'x', source: 'flagSettings', tokens: 1 }],
      skills: { totalSkills: 1, includedSkills: 1, tokens: 1, skillFrontmatter: [{ name: 'house', source: 'policySettings', tokens: 1 }] },
    } as never,
    home: '/home/user',
    root: '/repos/a',
    rules: false,
    servers: { github: 'pending', slack: 'failed', 'plugin:productivity:asana': 'pending' },
    loaded: [],
  })
  expect(items).toEqual([
    { kind: 'skill', name: 'house', source: 'managed', tokens: 1 },
    { kind: 'subagent', name: 'x', source: 'managed', tokens: 1 },
    { kind: 'mcp_server', name: 'github', source: 'mcp', from: 'github', state: 'connected' },
    { kind: 'mcp_server', name: 'asana', source: 'plugin', from: 'productivity', state: 'pending' },
    { kind: 'mcp_server', name: 'slack', source: 'mcp', from: 'slack', state: 'failed' },
    { kind: 'tool', name: 'mcp__github__get_me', source: 'mcp', from: 'github', state: 'loaded' },
    { kind: 'injection', name: 'armadaSession', source: 'armada' },
  ])
})

test('a skill or subagent used is sourced from the listing, else by its prefix, and an unlisted engine agent is built in', () => {
  const breakdown = {
    agents: [{ agentType: 'reviewer', source: 'projectSettings', tokens: 1 }],
    skills: { totalSkills: 1, includedSkills: 1, tokens: 1, skillFrontmatter: [{ name: 'land', source: 'projectSettings', tokens: 1 }] },
  } as never
  expect(skillUseOrigin('land', breakdown)).toEqual({ source: 'repository' })
  expect(skillUseOrigin('/land', breakdown)).toEqual({ source: 'repository' })
  expect(skillUseOrigin('armada:walk', breakdown)).toEqual({ source: 'armada' })
  expect(skillUseOrigin('unlisted', breakdown)).toBeUndefined()
  expect(agentUseOrigin('reviewer', 'engine', breakdown)).toEqual({ source: 'repository' })
  expect(agentUseOrigin('Explore', 'engine', breakdown)).toEqual({ source: 'built_in' })
  expect(agentUseOrigin('judge', 'armada', breakdown)).toEqual({ source: 'armada' })
})
