import { expect, test } from 'claude-code/testing'

import { condensed, consultedFact, consultedTarget } from './consulted'

const CWD = '/repos/armada'

test('a web page, a doc and a ticket are named by what the call asked', () => {
  expect(consultedTarget(CWD, 'WebFetch', { url: 'https://example.com/spec' })).toBe('https://example.com/spec')
  expect(consultedTarget(CWD, 'WebFetch', { url: 'not an address' })).toBeUndefined()
  expect(consultedTarget(CWD, 'Read', { file_path: '/repos/armada/docs/concepts/a.md' })).toBe('docs/concepts/a.md')
  expect(consultedTarget(CWD, 'Read', { file_path: '/repos/armada/crates/a/src/lib.rs' })).toBeUndefined()
  expect(consultedTarget(CWD, 'Read', { file_path: '/etc/notes.md' })).toBeUndefined()
  expect(consultedTarget(CWD, 'Bash', { command: 'gh issue view 123 --json title' })).toBe('123')
  expect(consultedTarget(CWD, 'Bash', { command: 'bd show armada-a3f2' })).toBe('armada-a3f2')
  expect(consultedTarget(CWD, 'Bash', { command: 'git status' })).toBeUndefined()
  expect(consultedTarget(CWD, 'mcp__linear__get_issue', { id: 'ENG-1' })).toBe('ENG-1')
  expect(consultedTarget(CWD, 'mcp__github__get_issue', { owner: 'o', repo: 'r', issue_number: 4 })).toBe('o/r#4')
  expect(consultedTarget(CWD, 'mcp__armada-fleet__get_feature_page', { id: 'x' })).toBeUndefined()
  expect(consultedTarget(CWD, 'Edit', { file_path: '/repos/armada/docs/a.md' })).toBeUndefined()
})

test('a markdown result is its heading and its first sentence, past the numbering and the front matter', () => {
  const text = '     1→---\n     2→owner: me\n     3→---\n     4→# Start a Job\n     5→\n     6→A Job starts from a request. It then runs.\n'
  expect(condensed(text)).toEqual({ title: 'Start a Job', gist: 'A Job starts from a request.' })
})

test('a JSON result is its title and the first sentence of its description', () => {
  const text = JSON.stringify({ title: 'Cards lose focus', description: 'Focus is lost after a retry. Seen twice.' })
  expect(condensed(text)).toEqual({ title: 'Cards lose focus', gist: 'Focus is lost after a retry.' })
})

test('what is sent is cut to about three hundred characters, and a result that was not seen sends the call alone', () => {
  const long = `# ${'T'.repeat(300)}\n\n${'word '.repeat(200)}`
  const fact = consultedFact(CWD, 'WebFetch', { url: 'https://example.com/a' }, long)
  if (fact?.kind !== 'consulted') throw new Error('not told')
  expect((fact.title ?? '').length + (fact.gist ?? '').length).toBeLessThanOrEqual(302)
  expect(consultedFact(CWD, 'WebFetch', { url: 'https://example.com/a' }, '')).toEqual({
    kind: 'consulted',
    tool: 'WebFetch',
    target: 'https://example.com/a',
  })
})
