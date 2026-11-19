// What the session read from outside the code, as a fact for Fleet's feature library: a page, an MCP server's answer,
// a ticket, a doc in the repository. Fleet says which of these are docs and tickets and files one only where a later
// write of the session reaches a feature, so this sends what the call named and a short cut of its result.
// **A title and one sentence, never the content.** Pure, so each reading is a test and not a session.

import { touchedOf } from './facts'
import type { Fact } from './fleet'

const TITLE_MOST = 100
const GIST_MOST = 200

/** The fields an MCP read names its document or ticket by, in the order tried. */
const BY = ['url', 'uri', 'id', 'issueIdOrKey', 'issue_key', 'issueKey', 'key', 'identifier', 'page_id', 'pageId', 'path']

/** The fields of a JSON result that read as its title and as its body, in the order tried. */
const TITLE_OF = ['title', 'name', 'subject', 'summary']
const BODY_OF = ['description', 'body', 'content', 'text', 'summary']

function cut(text: string, most: number): string {
  const one = text.replace(/\s+/g, ' ').trim()
  return one.length > most ? `${one.slice(0, most).trimEnd()}…` : one
}

/** What the call named: an address, a path from the session's directory, or an id. Undefined for a call that is no read. */
export function consultedTarget(cwd: string, tool: string, input: Record<string, unknown>): string | undefined {
  if (tool === 'WebFetch') return typeof input.url === 'string' && /^https?:\/\//.test(input.url) ? input.url : undefined
  if (tool === 'Read') {
    const path = touchedOf(cwd, input)
    return path !== undefined && (path.startsWith('docs/') || /\.mdx?$/i.test(path)) ? path : undefined
  }
  if (tool === 'Bash') {
    const line = typeof input.command === 'string' ? (input.command.trim().split('\n')[0] ?? '') : ''
    const words = line.split(/\s+/)
    const shown = words[0] === 'gh' && words[1] === 'issue' && words[2] === 'view' ? 3 : words[0] === 'bd' && words[1] === 'show' ? 2 : -1
    return shown < 0 ? undefined : words.slice(shown).find(word => !word.startsWith('-'))
  }
  if (!tool.startsWith('mcp__') || /^mcp__(armada|gitnexus)/.test(tool)) return undefined
  if (typeof input.owner === 'string' && typeof input.repo === 'string' && input.issue_number !== undefined) {
    return `${input.owner}/${input.repo}#${String(input.issue_number)}`
  }
  for (const field of BY) {
    const value = input[field]
    if (typeof value === 'string' && value !== '') return value
    if (typeof value === 'number') return String(value)
  }
  return undefined
}

/** The first sentence of prose in `lines`, past a heading, a front-matter block, a fence and a bare tag. */
function firstSentence(lines: string[]): string {
  let fenced = false
  let front = lines[0]?.trim() === '---'
  for (const [at, raw] of lines.entries()) {
    const line = raw.trim()
    if (front) {
      front = at === 0 || line !== '---'
      continue
    }
    if (line.startsWith('```')) fenced = !fenced
    if (fenced || line === '' || /^(#{1,6}\s|```|---|\|)/.test(line) || /^<\/?[a-z-]+[^>]*>$/i.test(line)) continue
    const prose = line.replace(/^(>|[-*]|\d+\.)\s+/, '')
    return cut(/^(.+?[.!?])(\s|$)/.exec(prose)?.[1] ?? prose, GIST_MOST)
  }
  return ''
}

/** A result as a title and a first sentence. Either is empty where the result held none. */
export function condensed(text: string): { title: string; gist: string } {
  // A Read answers with numbered lines, `   12→text`.
  const plain = text.replace(/^[ \t]*\d+[→\t]/gm, '')
  const start = plain.trimStart()
  if (start.startsWith('{')) {
    try {
      const found = JSON.parse(start) as Record<string, unknown>
      const pick = (fields: string[]): string => {
        for (const field of fields) if (typeof found[field] === 'string' && found[field] !== '') return found[field] as string
        return ''
      }
      return { title: cut(pick(TITLE_OF), TITLE_MOST), gist: firstSentence(pick(BODY_OF).split('\n')) }
    } catch {
      // Cut off or not whole: read it as text.
    }
  }
  const named =
    /^#{1,6}\s+(.+)$/m.exec(plain)?.[1] ??
    /<title>([^<]+)<\/title>/i.exec(plain)?.[1] ??
    /^title:\s*(.+)$/im.exec(plain)?.[1] ??
    /"title"\s*:\s*"([^"]+)"/.exec(plain)?.[1] ??
    ''
  return { title: cut(named.replace(/^["']|["']$/g, ''), TITLE_MOST), gist: firstSentence(plain.split('\n')) }
}

/** The read a call made, or undefined where it was none. `text` is the result, which may be empty where it was not seen. */
export function consultedFact(cwd: string, tool: string, input: Record<string, unknown>, text: string): Fact | undefined {
  const target = consultedTarget(cwd, tool, input)
  if (target === undefined) return undefined
  const { title, gist } = condensed(text.slice(0, 4000))
  return { kind: 'consulted', tool, target, ...(title === '' ? {} : { title }), ...(gist === '' ? {} : { gist }) }
}
