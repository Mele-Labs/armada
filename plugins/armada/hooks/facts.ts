// What a session's own words and tool results say about what it holds. Pure, so
// each reading is a test and not a session.

import type { SessionContextBreakdown } from 'claude-code'

import type { Fact, Task } from './fleet'

const TITLE_MOST = 80
const SAID_MOST = 200
const COMMAND_MOST = 300

/**
 * The version this mod reports to Fleet, so Bridge can mark a session whose mod is older than the
 * repository's. **Bump it with `version` in `.claude-plugin/plugin.json`**: the mod cannot read that
 * file while it runs, and a Fleet test (`terminal_session.rs`) holds the two equal.
 */
export const MOD_VERSION = '0.3.23'

/** What fills the context window, trimmed for Fleet (`ContextBreakdown`, `crates/ipc/src/context_breakdown.rs`). */
export type ContextBreakdown = {
  categories: { name: string; tokens: number; kind: 'used' | 'free' | 'buffer' | 'deferred' }[]
  memory_files: { path: string; tokens: number }[]
  mcp_tools: { name: string; server: string; tokens: number }[]
  total_tokens: number
  max_tokens: number
  autocompact_threshold?: number
}

const MOST_MCP_TOOLS = 10

/**
 * The breakdown as Fleet keeps it. **Names, paths and counts only**: the grid, the skill and agent
 * listings and every other field are left behind, and the MCP tools are the ten that weigh most.
 */
export function breakdownOf(breakdown: SessionContextBreakdown): ContextBreakdown {
  const trimmed: ContextBreakdown = {
    categories: breakdown.categories.map(one => ({ name: one.name, tokens: one.tokens, kind: one.kind })),
    memory_files: breakdown.memoryFiles.map(one => ({ path: one.path, tokens: one.tokens })),
    mcp_tools: [...breakdown.mcpTools]
      .sort((a, b) => b.tokens - a.tokens)
      .slice(0, MOST_MCP_TOOLS)
      .map(one => ({ name: one.name, server: one.serverName, tokens: one.tokens })),
    total_tokens: breakdown.totalTokens,
    max_tokens: breakdown.maxTokens,
  }
  if (breakdown.autoCompactThreshold !== undefined) trimmed.autocompact_threshold = breakdown.autoCompactThreshold
  return trimmed
}

/** A plan window as the engine reports it (`SessionRateLimit`). */
export type RateLimit = { kind: string; percentUsed: number; resetsAt?: string }

/** A plan window as Fleet takes it: whole percent, and the reset as an ISO instant Fleet can read. */
export type PlanWindow = { kind: string; percent_used: number; resets_at?: string }

function isoOf(text: string | undefined): string | undefined {
  if (text === undefined) return undefined
  const at = new Date(text)
  return Number.isNaN(at.getTime()) ? undefined : at.toISOString()
}

/** The windows in Fleet's spelling. Empty off a subscription, and a window with no usable reset keeps its percent. */
export function planOf(limits: RateLimit[]): PlanWindow[] {
  return limits.map(one => {
    const resets = isoOf(one.resetsAt)
    return { kind: one.kind, percent_used: Math.round(one.percentUsed), ...(resets === undefined ? {} : { resets_at: resets }) }
  })
}

/** Near enough to full that a turn failing on the limit was this window's doing. */
const NEARLY_FULL = 99

/**
 * When the limit a turn just hit opens again: the latest reset among the windows at or near full, so
 * a weekly window spent beside a five-hour one is waited out. Nothing where no full window says when.
 */
export function spentUntil(limits: RateLimit[]): string | undefined {
  const resets = limits
    .filter(one => one.percentUsed >= NEARLY_FULL)
    .map(one => isoOf(one.resetsAt))
    .filter((one): one is string => one !== undefined)
    .sort()
  return resets.at(-1)
}

/** The `answers` map a question's answered input carries, keyed by question text. */
export function answersIn(input: unknown): Record<string, string> {
  const answers = (input as { answers?: unknown } | null)?.answers
  if (typeof answers !== 'object' || answers === null) return {}
  return Object.fromEntries(Object.entries(answers).filter(([, said]) => typeof said === 'string')) as Record<string, string>
}

/**
 * The web pages a shell line opens in the owner's browser: `open`, `xdg-open` or
 * `python -m webbrowser` with an `http(s)` address. Fleet reads hosted sessions' lines the same way
 * (`pages_opened`, `crates/fleet/src/helm/deciding.rs`).
 */
export function pagesOpenedIn(command: string): string[] {
  const pages: string[] = []
  for (const segment of command.replace(/&&|\|\|/g, ';').split(/[;|\n]/)) {
    const words = segment
      .split(/\s+/)
      .filter(word => word !== '')
      .map(word => word.replace(/^['"]+|['"]+$/g, ''))
    const first = words[0] ?? ''
    const opens =
      first === 'open' ||
      first === 'xdg-open' ||
      (first.startsWith('python') && words.some((word, i) => word === '-m' && words[i + 1] === 'webbrowser'))
    if (opens) pages.push(...words.filter(word => /^https?:\/\//.test(word)))
  }
  return pages
}

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

/**
 * The first line of what a turn ended on, which Fleet shows as the reply: the first line of real text,
 * flattened to one line and no longer than a card's line holds. Nothing for a turn that said nothing.
 */
export function saidOf(answer: string): string | undefined {
  const line = answer
    .split('\n')
    .map(one => one.trim())
    .find(one => one !== '')
  if (line === undefined) return undefined
  const flat = line.replace(/\s+/g, ' ')
  return flat.length > SAID_MOST ? `${flat.slice(0, SAID_MOST - 1)}…` : flat
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
  // `--auto` asks the forge to merge later, or puts it in the merge queue: nothing has merged
  // yet, and Fleet's pull watch settles it when it does (#2027 read as merged, 8 Oct 2026).
  if (hit[1] === 'merge' && /(?:^|\s)--auto(?:\s|$)/.test(hit[2])) return undefined
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

export type Artifact = {
  /** A published page, a file written outside the code, a picture looked at, or a Claude Docs document. */
  form: 'page' | 'file' | 'image' | 'doc'
  /** The address a page or doc opens at, or the file's path. */
  target: string
  title?: string
}

// Written, and not code: documents, pictures and prose, never configuration. A page's own source (`.html`) is left
// out because the page appears once it is published, as a page.
const DOCUMENT = /\.(md|mdx|txt|pdf|png|jpe?g|gif|webp|svg|csv|docx?|xlsx?|pptx?)$/i
// Where a session keeps what it only needs on the way: its own settings and scratch space.
const SCRATCH = /(^|\/)(node_modules|\.git|\.claude|\.armada|target|dist|build)\/|^\/(private\/)?(tmp|var\/folders)\//

/** Whether a file a session wrote is something a person would want to open, and not code. */
export function isDocument(path: string): boolean {
  return DOCUMENT.test(path) && !SCRATCH.test(path)
}

// A picture a session looked at. Scratch is kept: a screenshot is usually read from `/tmp`.
const PICTURE = /\.(png|jpe?g|gif|webp)$/i

const CLAUDE_ADDRESS = /https:\/\/claude\.ai\/[^\s)"'<>\]]*artifact[^\s)"'<>\]]*/
const DOCS = /^mcp__claude_ai_Claude_Docs__(create|batch|update)$/

const nameOf = (path: string) => path.slice(path.lastIndexOf('/') + 1).replace(/\.[^.]+$/, '')

/**
 * What a tool call made, or looked at, that a person would open. **A code edit is never one**: only a page
 * published, a new document written, and a Claude Docs document created or edited.
 * `created` is whether a Write made a file that was not there.
 */
export function artifactOf(
  tool: string,
  input: Record<string, unknown>,
  text: string,
  created: boolean,
): Artifact | undefined {
  if (tool === 'Artifact') {
    if (input.asset === true || (input.action !== undefined && input.action !== 'publish')) return undefined
    const target = typeof input.url === 'string' ? input.url : CLAUDE_ADDRESS.exec(text)?.[0]
    if (target === undefined) return undefined
    const title =
      typeof input.title === 'string' ? input.title : typeof input.file_path === 'string' ? nameOf(input.file_path) : undefined
    return { form: 'page', target, title }
  }
  if (tool === 'Write') {
    const path = input.file_path
    if (!created || typeof path !== 'string' || !isDocument(path)) return undefined
    return { form: 'file', target: path, title: path.slice(path.lastIndexOf('/') + 1) }
  }
  if (tool === 'Read') {
    const path = input.file_path
    if (typeof path !== 'string' || !PICTURE.test(path)) return undefined
    return { form: 'image', target: path, title: path.slice(path.lastIndexOf('/') + 1) }
  }
  if (DOCS.test(tool)) {
    const container = input.container as { id?: unknown; create?: { name?: unknown } } | undefined
    const id = typeof container?.id === 'string' ? container.id : undefined
    const target = CLAUDE_ADDRESS.exec(text)?.[0] ?? (id === undefined ? undefined : `https://claude.ai/artifact/${id}`)
    if (target === undefined) return undefined
    const name = container?.create?.name
    return { form: 'doc', target, title: typeof name === 'string' ? name : undefined }
  }
  return undefined
}

/**
 * The file a write tool changed, named from the session's directory, for Fleet's check of who else
 * writes it. **A path and nothing else**: never the content, and nothing outside the directory.
 * `cwd` is where the session stands; an absolute path under it is made relative, and a path that
 * climbs out of it, or lies elsewhere, is not told.
 */
export function touchedOf(cwd: string, input: Record<string, unknown>): string | undefined {
  const named = input.file_path ?? input.notebook_path
  if (typeof named !== 'string' || named === '') return undefined
  const root = cwd.replace(/\/+$/, '')
  const relative = named.startsWith('/') ? (named.startsWith(`${root}/`) ? named.slice(root.length + 1) : undefined) : named
  if (relative === undefined || relative === '' || relative.split('/').includes('..')) return undefined
  return relative.replace(/^\.\//, '')
}

/**
 * What a tool call did that Fleet's timeline counts: a file it wrote or read, as a path from the
 * session's directory, or a shell command, as its first line. **Never content.** Fleet cuts a command
 * further and hides what looks like a secret in it (`crates/fleet/src/timeline/tools.rs`).
 */
export function usedFact(act: 'touched' | 'read' | 'ran', cwd: string, input: Record<string, unknown>): Fact | undefined {
  if (act === 'ran') {
    const line = typeof input.command === 'string' ? (input.command.trim().split('\n')[0] ?? '') : ''
    return line === '' ? undefined : { kind: 'ran', command: line.slice(0, COMMAND_MOST) }
  }
  const path = touchedOf(cwd, input)
  if (path === undefined) return undefined
  return act === 'read' ? { kind: 'read', path } : { kind: 'touched', paths: [path] }
}

/**
 * What a permission prompt asks about, as the call names it: a shell command's first line, a file's
 * path, an address, or nothing for a tool with none of these. One line and cut, never content. The
 * same reading of the prompt's input and of the tool call's, so the call settles the prompt it raised.
 */
export function promptDetail(input: Record<string, unknown>): string {
  const named = [input.command, input.file_path, input.notebook_path, input.path, input.url].find(
    one => typeof one === 'string' && one.trim() !== '',
  )
  if (typeof named !== 'string') return ''
  return (named.trim().split('\n')[0] ?? '').slice(0, COMMAND_MOST)
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

const TASK_MOST = 200

function textOf(value: unknown): string | undefined {
  if (typeof value !== 'string') return undefined
  const text = value.replace(/\s+/g, ' ').trim()
  return text === '' ? undefined : text.slice(0, TASK_MOST)
}

function statusOf(value: unknown): Task['status'] | undefined {
  return value === 'pending' || value === 'in_progress' || value === 'completed' ? value : undefined
}

/**
 * The list a `TodoWrite` wrote, whole: its items in order, each with its place as the id. An input
 * with no list is nothing to tell, and an empty one is a list cleared.
 */
export function todoTasks(input: Record<string, unknown>): Task[] | undefined {
  if (!Array.isArray(input.todos)) return undefined
  const tasks: Task[] = []
  for (const one of input.todos as Record<string, unknown>[]) {
    const text = textOf(one?.content ?? one?.subject)
    if (text === undefined) continue
    tasks.push({ id: String(tasks.length + 1), text, status: statusOf(one.status) ?? 'pending' })
  }
  return tasks
}

/** The id the task tool gave a task it made, from its result: `{ task: { id } }`, or `Task #3 created`. */
function madeId(result: unknown, text: string): string | undefined {
  const task = (result as { task?: { id?: unknown } } | null | undefined)?.task
  if (typeof task?.id === 'string' || typeof task?.id === 'number') return String(task.id)
  return /\bTask #?(\w+)/.exec(text)?.[1]
}

/** The list after a `TaskCreate`, or nothing where the call named no task. */
export function madeTask(list: Task[], input: Record<string, unknown>, result: unknown, text: string): Task[] | undefined {
  const named = textOf(input.subject ?? input.content ?? input.title)
  if (named === undefined) return undefined
  const id = madeId(result, text) ?? String(list.length + 1)
  return [...list.filter(one => one.id !== id), { id, text: named, status: 'pending' }]
}

/**
 * The list after a `TaskUpdate`. A task the mod never saw made (a session resumed in a terminal) is
 * not in it, and is not invented: nothing to tell.
 */
export function updatedTask(list: Task[], input: Record<string, unknown>): Task[] | undefined {
  const id = input.taskId ?? input.task_id ?? input.id
  const key = typeof id === 'string' || typeof id === 'number' ? String(id) : undefined
  const was = list.find(one => one.id === key)
  if (key === undefined || was === undefined) return undefined
  if (input.status === 'deleted') return list.filter(one => one.id !== key)
  const next: Task = {
    id: key,
    text: textOf(input.subject ?? input.content ?? input.title) ?? was.text,
    status: statusOf(input.status) ?? was.status,
  }
  if (next.text === was.text && next.status === was.status) return undefined
  return list.map(one => (one.id === key ? next : one))
}

/**
 * A successful `git commit`, read from its output (`[branch 1a2b3c4] subject`) and not from the command alone: a
 * commit with nothing staged prints no such line. `branch` is absent on a detached head.
 */
export function commitIn(command: string, output: string): { branch?: string } | undefined {
  if (!/\bgit\b[^|;&\n]*\bcommit\b/.test(command)) return undefined
  const made = /^\[(\S+)(?: [^\]]*)? [0-9a-f]{7,40}\]/m.exec(output)
  if (made === null) return undefined
  return made[1] === 'detached' ? {} : { branch: made[1] }
}

/**
 * The branch a successful `git push` updated, from the ref line it printed: `abc..def  main -> main`,
 * `* [new branch]  topic -> topic`, a forced `+ abc...def`. A push with nothing to send, one that was
 * rejected and a `--dry-run` print no such line or are not one, and tell nothing.
 */
export function pushIn(command: string, output: string): { branch?: string } | undefined {
  if (!/\bgit\b[^|;&\n]*\bpush\b/.test(command) || /--dry-run|(?:^|\s)-n(?:\s|$)/.test(command)) return undefined
  const sent = /^\s*(?:[0-9a-f]{7,40}\.\.\.?[0-9a-f]{7,40}|\*\s+\[new (?:branch|tag)\]|\+\s+[0-9a-f]{7,40}\.\.\.[0-9a-f]{7,40})\s+\S+\s+->\s+(\S+)/m.exec(output)
  return sent === null ? undefined : { branch: sent[1] }
}
