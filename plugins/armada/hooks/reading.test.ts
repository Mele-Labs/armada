import { expect, test } from 'claude-code/testing'

import { outlineOf, turnedAway, wholeRead, WHOLE_READ_MOST } from './reading'

const BIG = '/repos/armada/docs/practices/protocol.md'
const DOC = ['# Protocol', '', 'Prose.', '```sh', '# not a heading', '```', '## The ID', 'More prose.'].join('\n')

test('a heading is outlined with the line Read would start at, and a comment in a fence is not', () => {
  expect(outlineOf('notes.md', DOC)).toEqual(['1: # Protocol', '7: ## The ID'])
})

test('code is outlined by its top-level declarations', () => {
  const rust = ['use x;', '', 'pub fn serve() {', '    fn inner() {}', '}', 'pub(crate) struct Held;', 'impl Held {}'].join('\n')
  expect(outlineOf('serving.rs', rust)).toEqual(['3: pub fn serve() {', '6: pub(crate) struct Held;', '7: impl Held {}'])
  expect(outlineOf('door.ts', 'import x\nexport const A = 1\nconst b = 2\n')).toEqual(['2: export const A = 1', '3: const b = 2'])
})

test('only a whole read of a file read as lines is looked at', () => {
  expect(wholeRead({ file_path: BIG })).toBe(true)
  expect(wholeRead({ file_path: BIG, offset: 10 })).toBe(false)
  expect(wholeRead({ file_path: BIG, limit: 50 })).toBe(false)
  expect(wholeRead({ file_path: BIG, allow_large: true })).toBe(false)
  expect(wholeRead({ file_path: '/repos/armada/spec.pdf' })).toBe(false)
  expect(wholeRead({ file_path: '/repos/armada/AppIcon.icns' })).toBe(false)
})

test('the agent is told the size, how to read a range, and where things are', () => {
  const told = turnedAway(BIG, 379 * 1024, DOC)
  expect(told).toContain('379 KB, 8 lines, about 97k tokens')
  expect(told).toContain('offset and limit')
  expect(told).toContain('7: ## The ID')
})

test('a large file is turned away once with its outline, and the second whole read goes through', async ($, on) => {
  const reads: string[] = []
  on('fs.stat', () => ({ value: { kind: 'file' as const, size: WHOLE_READ_MOST + 1, mtimeMs: 0, isLink: false } }))
  on('fs.read', () => ({ value: DOC }))
  on('tool.call', { tool: 'Read' }, (_$, e) => {
    reads.push(e.file_path)
    return { result: {}, text: DOC }
  })

  const first = await $.tool.call({ tool: 'Read', file_path: BIG })
  expect(first.deny).toContain('7: ## The ID')
  expect(reads).toEqual([])

  const second = await $.tool.call({ tool: 'Read', file_path: BIG })
  expect(second.deny).toBeUndefined()
  expect(reads).toEqual([BIG])
})

test('a small file, or a ranged read of a large one, is read as asked', async ($, on) => {
  let size = 1024
  const reads: string[] = []
  on('fs.stat', () => ({ value: { kind: 'file' as const, size, mtimeMs: 0, isLink: false } }))
  on('fs.read', () => ({ value: DOC }))
  on('tool.call', { tool: 'Read' }, (_$, e) => {
    reads.push(e.file_path)
    return { result: {}, text: DOC }
  })

  expect((await $.tool.call({ tool: 'Read', file_path: '/repos/armada/small.md' })).deny).toBeUndefined()
  size = WHOLE_READ_MOST * 10
  expect((await $.tool.call({ tool: 'Read', file_path: '/repos/armada/big.md', offset: 100, limit: 50 })).deny).toBeUndefined()
  expect(reads).toEqual(['/repos/armada/small.md', '/repos/armada/big.md'])
})
