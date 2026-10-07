// What a session's own words and tool results say about what it holds. Pure, so
// each reading is a test and not a session.

const TITLE_MOST = 80

// A harness wraps what it adds to a prompt in hyphenated tags. A command's and a reminder's
// contents are the harness's words, so the whole block goes; any other wrapper, such as
// `<agent-message from="…">`, only loses its tag and the words inside are the person's.
const MACHINE_BLOCK = /<((?:local-)?command-[\w-]+|system-reminder)(?:\s[^>]*)?>[\s\S]*?(?:<\/\1>|$)/g
const WRAPPER_TAG = /<\/?[a-z]+(?:-[\w]+)+(?:\s[^>]*)?>/gi

/** What is left of a prompt once the harness's own markup is taken off. */
export function withoutMarkup(prompt: string): string {
  return prompt.replace(MACHINE_BLOCK, '').replace(WRAPPER_TAG, '')
}

/** The first line of real text in a prompt as a title, or nothing for a command or a blank. */
export function titleOf(prompt: string): string | undefined {
  const line = withoutMarkup(prompt)
    .split('\n')
    .map(one => one.trim())
    .find(one => one !== '')
  if (line === undefined || line.startsWith('/')) return undefined
  const flat = line.replace(/\s+/g, ' ')
  return flat.length > TITLE_MOST ? `${flat.slice(0, TITLE_MOST - 1)}…` : flat
}

/** The name a `/rename` gave, from a transcript: its last `custom-title` entry. */
export function customTitleIn(transcript: string): string | undefined {
  let found: string | undefined
  for (const line of transcript.split('\n')) {
    if (!line.includes('customTitle')) continue
    try {
      const entry = JSON.parse(line) as { type?: unknown; customTitle?: unknown }
      if (entry.type === 'custom-title' && typeof entry.customTitle === 'string') {
        found = entry.customTitle.trim() || found
      }
    } catch {
      // A line cut short at the end of a file being written.
    }
  }
  return found === undefined ? undefined : found.replace(/\s+/g, ' ')
}

/** Where Claude Code keeps a session's transcript, under the directory it ran in. */
export function transcriptPath(home: string, cwd: string, id: string): string {
  return `${home}/.claude/projects/${cwd.replace(/[^a-zA-Z0-9]/g, '-')}/${id}.jsonl`
}

/** Whether a Bash command may have moved the branch or the directory. */
export function mayMoveBranch(command: string): boolean {
  return /\bgit\b[^|;&]*\b(checkout|switch|branch|commit|merge|rebase|pull|worktree|reset|cherry-pick|clone)\b/.test(
    command,
  )
}

export type PullRequest = { number: string; url: string }

/** Pull requests a result names by their address. */
export function pullRequestsIn(text: string): PullRequest[] {
  const found = new Map<string, PullRequest>()
  for (const hit of text.matchAll(/https:\/\/github\.com\/[\w.-]+\/[\w.-]+\/pull\/(\d+)/g)) {
    found.set(hit[1], { number: hit[1], url: hit[0] })
  }
  return [...found.values()]
}

export type GhAct = { act: 'create' | 'merge' | 'close'; number?: string }

/** A `gh pr` command that makes, lands or closes a pull request. */
export function ghAct(command: string): GhAct | undefined {
  const hit = /\bgh\s+pr\s+(create|merge|close)\b([^|;&]*)/.exec(command)
  if (hit === null) return undefined
  const number = /(?:^|\s)#?(\d+)(?:\s|$)/.exec(hit[2])?.[1]
  return { act: hit[1] as GhAct['act'], number }
}

export type NeedAct =
  | { act: 'declare'; path: string; what: string }
  | { act: 'took'; path: string; value: string }
  | { act: 'release'; path: string }

/** The words of a shell command, quotes taken off. */
function wordsOf(text: string): string[] {
  return [...text.matchAll(/"((?:[^"\\]|\\.)*)"|'([^']*)'|(\S+)/g)].map(
    hit => hit[1]?.replace(/\\(.)/g, '$1') ?? hit[2] ?? hit[3],
  )
}

/** The `armada need` a command runs, in the three forms that declare, take or give back. */
export function needAct(command: string): NeedAct | undefined {
  const hit = /(?:^|[;&|(])\s*armada\s+need\s+([^|;&\n]*)/.exec(command)
  if (hit === null) return undefined
  const rest = wordsOf(hit[1])
  const path = (one: string) => one.replace(/^(\.\/)+/, '')
  if (rest[0] === '--release' && rest.length === 2) return { act: 'release', path: path(rest[1]) }
  if (rest[0] === '--took' && rest.length === 3) {
    return { act: 'took', path: path(rest[1]), value: rest[2] }
  }
  if (rest.length === 2 && !rest[0].startsWith('-')) {
    return { act: 'declare', path: path(rest[0]), what: rest[1] }
  }
  return undefined
}

const DISPATCH = /^mcp__armada-fleet__(propose_job|propose_from_request|approve_dispatch|redispatch_job)$/

export function isDispatch(tool: string): boolean {
  return DISPATCH.test(tool)
}

/** Job ids in a dispatch's answer, which are ULIDs. */
export function jobIdsIn(text: string): string[] {
  return [...new Set(text.match(/\b[0-9A-HJKMNP-TV-Z]{26}\b/g) ?? [])].slice(0, 10)
}

/** Whom a delivery came from, as far as the delivery says: never a session id. */
export function senderOf(origin: { kind: string; teammate?: string }): string {
  return origin.teammate ?? origin.kind
}

export function micros(usd: number | undefined): number | undefined {
  return usd === undefined ? undefined : Math.round(usd * 1_000_000)
}

const MODELS = ['haiku', 'sonnet', 'opus']

/** A model by the short name the person picks it by, and by its own id where it has no short one. */
export function modelName(id: string): string {
  return MODELS.find(one => id.includes(one)) ?? id
}

export type Mode = 'ask' | 'auto' | 'accept_edits' | 'plan'

/** The terminal's permission mode in Armada's words, or nothing for one Armada has no word for. */
export function modeOf(mode: string | undefined): Mode | undefined {
  switch (mode) {
    case 'default':
      return 'ask'
    case 'auto':
      return 'auto'
    case 'acceptEdits':
      return 'accept_edits'
    case 'plan':
      return 'plan'
    default:
      return undefined
  }
}

/** An effort level, whether the hook input carries the word or an object around it. */
export function effortOf(effort: unknown): string | undefined {
  if (typeof effort === 'string') return effort
  const level = (effort as { level?: unknown } | null | undefined)?.level
  return typeof level === 'string' ? level : undefined
}
