import { expect, mock, test } from 'claude-code/testing'
import type { On } from 'claude-code'

const RUNNING = '{"protocol_id":"0000000000000000","pid":1,"port":4242,"started_at":"x"}'
const PLAN = 'the person approves the plan'
const CHECKS = 'Checks are green at the pushed commit'
const MERGED = 'the branch is merged'

function gates(closed: string[]) {
  const one = (key: string, name: string) => ({ name, closed_at: closed.includes(key) ? '2026-11-07T00:00:00Z' : null })
  return JSON.stringify({ gates: { plan_approved: one('plan', PLAN), checks_green: one('checks', CHECKS), merged: one('merged', MERGED) } })
}

function world(on: On, record: string | undefined, tracked: boolean) {
  mock.clock(on, { now: 1_000_000 })
  mock.env(on, { HOME: '/home/user' })
  on('fs.read', (_$, e) => {
    if (e.path.endsWith('/.spike046/S1/record.json')) {
      if (record === undefined) throw new Error('no such file')
      return { value: record }
    }
    return { value: RUNNING }
  })
  on('fs.exists', () => ({ value: true }))
  on('process.run', () => ({ value: { exitCode: tracked ? 0 : 1, stdout: '', stderr: '' } }))
  on('session.id', () => ({ value: 'S1' }))
  on('http.fetch', () => ({ value: { status: 200, ok: true, headers: {}, text: '{}' } }))
  on('tool.call', { tool: 'Edit' }, () => ({ result: {}, text: 'edited' }))
  on('tool.call', { tool: 'Write' }, () => ({ result: {}, text: 'written' }))
  on('tool.call', { tool: 'Bash' }, () => ({ result: { stdout: '', stderr: '', interrupted: false }, text: 'ran' }))
  on('tool.call', { tool: 'mcp__armada-fleet__all_done' }, () => ({ result: {}, text: 'done' }))
}

test('an edit of a tracked file before the plan gate closes is refused with the gate named', async ($, on) => {
  world(on, gates([]), true)
  const out = await $.tool.call({ tool: 'Edit', file_path: '/slot/apps/a.ts', old_string: 'a', new_string: 'b' })
  expect(out.deny).toBe(`Gate open: ${PLAN}.`)
})

test('a write under .armada and an untracked file are allowed before the plan gate closes', async ($, on) => {
  world(on, gates([]), false)
  expect((await $.tool.call({ tool: 'Write', file_path: '/slot/zz-scratch.ts', content: 'x' })).deny).toBeUndefined()
  expect((await $.tool.call({ tool: 'Write', file_path: '/slot/.armada/note.md', content: 'x' })).deny).toBeUndefined()
})

test('a tracked edit is allowed once the plan gate has closed', async ($, on) => {
  world(on, gates(['plan']), true)
  const out = await $.tool.call({ tool: 'Edit', file_path: '/slot/apps/a.ts', old_string: 'a', new_string: 'b' })
  expect(out.deny).toBeUndefined()
})

test('a git push before the plan gate closes is refused, a commit is not', async ($, on) => {
  world(on, gates([]), true)
  expect((await $.tool.call({ tool: 'Bash', command: 'git push origin armada/session-1' })).deny).toBe(`Gate open: ${PLAN}.`)
  expect((await $.tool.call({ tool: 'Bash', command: 'git commit -m x' })).deny).toBeUndefined()
})

test('all_done is refused while any gate is open and names each', async ($, on) => {
  world(on, gates(['plan']), true)
  const out = await $.tool.call({ tool: 'mcp__armada-fleet__all_done', summary: 'x' })
  expect(out.deny).toBe(`Gate open: ${CHECKS}; ${MERGED}.`)
})

test('all_done is allowed when every gate is closed', async ($, on) => {
  world(on, gates(['plan', 'checks', 'merged']), true)
  expect((await $.tool.call({ tool: 'mcp__armada-fleet__all_done', summary: 'x' })).deny).toBeUndefined()
})

test('a session with no record is never refused', async ($, on) => {
  world(on, undefined, true)
  expect((await $.tool.call({ tool: 'mcp__armada-fleet__all_done', summary: 'x' })).deny).toBeUndefined()
  expect((await $.tool.call({ tool: 'Edit', file_path: '/slot/apps/a.ts', old_string: 'a', new_string: 'b' })).deny).toBeUndefined()
})
