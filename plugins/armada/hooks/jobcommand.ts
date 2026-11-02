// `armada job` as the mod reads it from a Bash call. The Rust half is `crates/fleet/src/session_host/
// job_command.rs`, and the two hold the same rules: a line is one `armada job` command or it is the
// shell's. Quotes are followed, so a JSON body's `;` and `|` are text; what the shell would act on is not.
import { waitingItemsOf } from './facts'
import type { WaitingItem } from './facts'

type JobCommand = { words: string[]; heredoc?: string }

// Unquoted, any of these is the shell doing something besides the one command.
const SHELL = new Set([';', '|', '&', '>', '$', '`', '\\', '\n', '\r', '(', ')'])

/** The body a heredoc carries, where its delimiter line ends it and nothing but blank space follows. */
function heredocBody(after: string, tag: string): string | undefined {
  const lines = after.split('\n')
  const end = lines.indexOf(tag)
  if (end < 0 || !lines.slice(end + 1).every(line => line.trim() === '')) return undefined
  return lines.slice(0, end).join('\n')
}

/** The words of a line that is exactly one `armada job` command, and its heredoc where it has one. */
export function readJobCommand(line: string): JobCommand | undefined {
  const words: string[] = []
  let word = ''
  let inWord = false
  let heredoc: string | undefined
  for (let at = 0; at < line.length; at += 1) {
    const c = line[at]!
    if (c === "'") {
      const end = line.indexOf("'", at + 1)
      if (end < 0) return undefined
      word += line.slice(at + 1, end)
      inWord = true
      at = end
    } else if (c === '"') {
      let end = at + 1
      for (; line[end] !== '"'; end += 1) {
        const d = line[end]
        if (d === undefined || d === '$' || d === '`' || d === '\\') return undefined
        word += d
      }
      inWord = true
      at = end
    } else if (c === ' ' || c === '\t') {
      if (inWord) words.push(word)
      word = ''
      inWord = false
    } else if (c === '<') {
      const opens = /^<<'([A-Za-z0-9_]+)'[ \t]*\n/.exec(line.slice(at))
      if (opens === null) return undefined
      heredoc = heredocBody(line.slice(at + opens[0].length), opens[1]!)
      if (heredoc === undefined) return undefined
      if (inWord) words.push(word)
      inWord = false
      break
    } else if (SHELL.has(c)) {
      return undefined
    } else {
      word += c
      inWord = true
    }
  }
  if (inWord) words.push(word)
  if (words[0] !== 'armada' || words[1] !== 'job') return undefined
  // The body is the act's, so the act is told to read stdin.
  if (heredoc !== undefined && words.at(-1) !== '-') return undefined
  return heredoc === undefined ? { words } : { words, heredoc }
}

/** Whether Bash would run exactly one `armada job` command, which asks nobody. */
export function isOneArmadaJob(line: unknown): boolean {
  return typeof line === 'string' && readJobCommand(line) !== undefined
}

/**
 * The list an `armada job waiting_for` command sets, from the JSON argument, a heredoc on `-`, or an
 * `items=[...]` field. Undefined where the line is not that command or names no list.
 */
export function waitingItemsOfJob(line: unknown): WaitingItem[] | undefined {
  if (typeof line !== 'string') return undefined
  const read = readJobCommand(line)
  if (read === undefined || read.words[2]?.replaceAll('-', '_') !== 'waiting_for') return undefined
  for (const word of read.words.slice(3)) {
    const body = word === '-' ? read.heredoc : word.startsWith('{') ? word : word.startsWith('items=') ? `{"items":${word.slice(6)}}` : undefined
    const items = body === undefined ? undefined : waitingItemsOf({ body })
    if (items !== undefined) return items
  }
  return undefined
}
