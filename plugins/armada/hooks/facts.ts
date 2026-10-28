// What a session's own words and tool results say about what it holds. Pure, so
// each reading is a test and not a session.

import type { SessionContextBreakdown } from 'claude-code'

import type { Fact, Task } from './fleet'

const TITLE_MOST = 80
const SAID_MOST = 1500
const COMMAND_MOST = 300

/**
 * The version this mod reports to Fleet, so Bridge can mark a session whose mod is older than the
 * repository's. **Bump it with `version` in `.claude-plugin/plugin.json`**: the mod cannot read that
 * file while it runs, and a Fleet test (`terminal_session.rs`) holds the two equal.
 */
export const MOD_VERSION = '0.3.50'

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

/** A browser the agent drove outside Bridge: `tool` as the owner reads it, and the address the call named. */
export type OutsideBrowser = { tool: 'cmux' | 'Playwright'; address?: string }

/**
 * Whether a shell line drives cmux's browser (`cmux browser ...`, behind `&&`, `;` or env prefixes),
 * and the first http(s) address it names. A test suite's `playwright test` is not a browser the owner could share.
 */
export function cmuxBrowserIn(command: string): OutsideBrowser | undefined {
  for (const segment of command.replace(/&&|\|\|/g, ';').split(/[;|\n]/)) {
    const words = segment
      .split(/\s+/)
      .filter(word => word !== '')
      .map(word => word.replace(/^['"]+|['"]+$/g, ''))
    while (/^\w+=/.test(words[0] ?? '')) words.shift()
    if (words[0] !== 'cmux') continue
    const rest = words.slice(1).filter(word => !word.startsWith('-'))
    if (rest[0] !== 'browser') continue
    const address = words.find(word => /^https?:\/\//.test(word))
    return address === undefined ? { tool: 'cmux' } : { tool: 'cmux', address }
  }
  return undefined
}

/** Whether a tool call reaches for a browser outside Bridge: cmux's skill, or Playwright's browser tools over MCP. */
export function outsideBrowserOf(tool: string, input: Record<string, unknown>): OutsideBrowser | undefined {
  if (tool === 'Skill') return /(^|:)cmux-browser$/.test(String(input.skill)) ? { tool: 'cmux' } : undefined
  if (!/^mcp__.*playwright.*__browser_/i.test(tool)) return undefined
  return typeof input.url === 'string' && input.url !== '' ? { tool: 'Playwright', address: input.url } : { tool: 'Playwright' }
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
 * The close of what a turn ended on, which Fleet reads for an ask and shows the first line of as the reply:
 * the last 1,500 characters from the start of a line, as Fleet keeps them for a hosted Session (`tail` in
 * `crates/fleet/src/session_host/standing.rs`, which this mirrors). Nothing for a turn that said nothing.
 */
export function saidOf(answer: string): string | undefined {
  const said = answer.trim()
  if (said === '') return undefined
  const chars = Array.from(said)
  if (chars.length <= SAID_MOST) return said
  const kept = chars.slice(chars.length - SAID_MOST).join('')
  const cut = kept.indexOf('\n')
  if (cut >= 0) {
    const rest = kept.slice(cut + 1)
    if (rest.trim() !== '') return rest.trimStart()
  }
  return kept
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

/** Issues a result names by their address, as `pullRequestsIn` reads pull requests. */
export function issuesIn(text: string): PullRequest[] {
  const found = new Map<string, PullRequest>()
  for (const hit of text.matchAll(/https:\/\/github\.com\/[\w.-]+\/[\w.-]+\/issues\/(\d+)/g)) {
    found.set(hit[1], { number: hit[1], url: hit[0] })
  }
  return [...found.values()]
}

/** A Bash command that files an issue with `gh`, which prints the new issue's address. */
export function ghIssueCreate(command: string): boolean {
  return /\bgh\s+issue\s+create\b/.test(command)
}

export type GhAct = { act: 'create' | 'merge' | 'close'; number?: string }

/** A `gh pr` command that makes, lands or closes a pull request. */
export function ghAct(command: string): GhAct | undefined {
  const hit = /\bgh\s+pr\s+(create|merge|close)\b([^|;&]*)/.exec(command)
  if (hit === null) return undefined
  // `--auto` asks the forge to merge later, or puts it in the merge queue: nothing has merged
  // yet, and Fleet's pull reading settles it when it does (#2027 read as merged, 8 Oct 2026).
  if (hit[1] === 'merge' && /(?:^|\s)--auto(?:\s|$)/.test(hit[2])) return undefined
  const number = /(?:^|\s)#?(\d+)(?:\s|$)/.exec(hit[2])?.[1]
  return { act: hit[1] as GhAct['act'], number }
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

/** A task the agent started in the background: a shell command run with `run_in_background`, or a Monitor watch. */
export type Background = { target: string; detail: Record<string, string> }

const SHELL_TASK = /running in background with ID:\s*([\w-]+)/
// **No Monitor result has been recorded.** This pattern is tolerant on purpose ("ID: x" or "task x", an id of five or
// more characters) and wants a real sample to tighten it. The shell's is verbatim (`background.fixture.ts`).
const MONITOR_TASK = /(?:\bID:?|\btask)\s+([A-Za-z0-9][\w-]{4,})/
// The sentence ends the path with a full stop, which is not part of it.
const OUTPUT_FILE = /Output is being written to:\s*(\S+?)\.?(?:\s|$)/

/**
 * A background task, from the call that started it and the id its result gave back. **No id in the result, no
 * task**: a call that was refused or answered in some other words is not reported. `task` is the call's own
 * description; `output` is the file the engine writes the task's output to, when the result names it.
 */
export function backgroundOf(tool: string, input: Record<string, unknown>, text: string): Background | undefined {
  const said = (value: unknown) => (typeof value === 'string' ? (value.trim().split('\n')[0] ?? '').slice(0, COMMAND_MOST) : '')
  const output = OUTPUT_FILE.exec(text)?.[1]
  const where = output === undefined ? {} : { output }
  if (tool === 'Bash') {
    const target = SHELL_TASK.exec(text)?.[1]
    if (input.run_in_background !== true || target === undefined) return undefined
    const command = said(input.command)
    return { target, detail: { form: 'command', task: said(input.description) || command, command, ...where } }
  }
  if (tool === 'Monitor') {
    const target = MONITOR_TASK.exec(text)?.[1]
    if (target === undefined) return undefined
    const watched = said(input.command) || said((input.ws as { url?: unknown } | undefined)?.url)
    return { target, detail: { form: 'monitor', task: said(input.description) || watched, command: watched, ...where } }
  }
  return undefined
}

/** How a background task ended, off the notification the engine queues for the session: the task's id, its word, and the exit code when the summary gives one. */
export type BackgroundEnd = { target: string; status: string; exit?: string; output?: string }

export function backgroundEndOf(text: string): BackgroundEnd | undefined {
  const field = (name: string) => new RegExp(`<${name}>\\s*([\\s\\S]*?)\\s*</${name}>`).exec(text)?.[1]
  const target = field('task-id')
  const status = field('status')
  if (target === undefined || status === undefined) return undefined
  const exit = /exit code\s+(-?\d+)/i.exec(field('summary') ?? '')?.[1]
  // The notice names the file the engine wrote the output to, which is better than any parsing of the launch.
  const output = field('output-file')
  return { target, status, ...(exit === undefined ? {} : { exit }), ...(output === undefined ? {} : { output }) }
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
export function usedFact(
  act: 'touched' | 'read' | 'ran',
  cwd: string,
  input: Record<string, unknown>,
  by?: { how?: 'write' | 'edit'; agent?: string },
): Fact | undefined {
  if (act === 'ran') {
    const line = typeof input.command === 'string' ? (input.command.trim().split('\n')[0] ?? '') : ''
    return line === '' ? undefined : { kind: 'ran', command: line.slice(0, COMMAND_MOST) }
  }
  const path = touchedOf(cwd, input)
  if (path === undefined) return undefined
  if (act === 'read') return { kind: 'read', path }
  // Which subagent wrote it, by the id its `subagent` attachment carries, and whether it made the file.
  return {
    kind: 'touched',
    paths: [path],
    ...(by?.how === undefined ? {} : { act: by.how }),
    ...(by?.agent === undefined ? {} : { by: by.agent }),
  }
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

/** The list after a `TaskCreate`, or nothing where the call named no task. `by` is the subagent that made it. */
export function madeTask(
  list: Task[],
  input: Record<string, unknown>,
  result: unknown,
  text: string,
  by?: string,
): Task[] | undefined {
  const named = textOf(input.subject ?? input.content ?? input.title)
  if (named === undefined) return undefined
  const id = madeId(result, text) ?? String(list.length + 1)
  return [...list.filter(one => one.id !== id), { id, text: named, status: 'pending', ...(by === undefined ? {} : { by }) }]
}

/**
 * Where Claude Code keeps a task list: `~/.claude/tasks/<list>/<n>.json`, one file a task. The list is the
 * session's id unless `CLAUDE_CODE_TASK_LIST_ID` names it, and the harness cleans the name the same way.
 */
export function taskListDir(home: string, list: string): string {
  return `${home}/.claude/tasks/${list.replace(/[^a-zA-Z0-9_-]/g, '-')}`
}

/**
 * The harness's own list, from its task files. It is what the agent's tools left, whichever process made
 * them, so a process that began after the list was written (a resumed session) starts from it. A file
 * that is not a task, or not whole yet, is left out. In order of id.
 */
export function storedTasks(files: string[]): Task[] {
  const tasks: Task[] = []
  for (const file of files) {
    let one: Record<string, unknown>
    try {
      one = JSON.parse(file) as Record<string, unknown>
    } catch {
      continue
    }
    const id = typeof one?.id === 'string' || typeof one?.id === 'number' ? String(one.id) : undefined
    const text = textOf(one.subject)
    if (id === undefined || text === undefined) continue
    tasks.push({ id, text, status: statusOf(one.status) ?? 'pending' })
  }
  return tasks.sort((a, b) => Number(a.id) - Number(b.id) || a.id.localeCompare(b.id))
}

/**
 * The list after a `TaskUpdate`. A task that is in neither the list nor the harness's store is not
 * invented: nothing to tell.
 */
export function updatedTask(list: Task[], input: Record<string, unknown>): Task[] | undefined {
  const id = input.taskId ?? input.task_id ?? input.id
  const key = typeof id === 'string' || typeof id === 'number' ? String(id) : undefined
  const was = list.find(one => one.id === key)
  if (key === undefined || was === undefined) return undefined
  if (input.status === 'deleted') return list.filter(one => one.id !== key)
  // Whoever updates it, the task stays its maker's.
  const next: Task = {
    ...was,
    text: textOf(input.subject ?? input.content ?? input.title) ?? was.text,
    status: statusOf(input.status) ?? was.status,
  }
  if (next.text === was.text && next.status === was.status) return undefined
  return list.map(one => (one.id === key ? next : one))
}

/**
 * A successful `git commit`, read from its output (`[branch 1a2b3c4] subject`) and not from the command alone: a
 * commit with nothing staged prints no such line. `branch` is absent on a detached head. `commit` is the hash as
 * printed and `subject` the line after it, which Fleet matches against the plan's tasks.
 */
export function commitIn(command: string, output: string): { branch?: string; commit: string; subject: string } | undefined {
  if (!/\bgit\b[^|;&\n]*\bcommit\b/.test(command)) return undefined
  const made = /^\[(\S+)(?: [^\]]*)? ([0-9a-f]{7,40})\] ?(.*)$/m.exec(output)
  if (made === null) return undefined
  const said = { commit: made[2], subject: made[3].trim() }
  return made[1] === 'detached' ? said : { branch: made[1], ...said }
}

/**
 * The branch a successful `git push` updated, from the ref line it printed: `abc..def  main -> main`,
 * `* [new branch]  topic -> topic`, a forced `+ abc...def`. A push with nothing to send, one that was
 * rejected and a `--dry-run` print no such line or are not one, and tell nothing. **A quiet push prints
 * nothing at all**, so one that succeeded is a push of the branch its refspec names (28 Oct 2026: a Job
 * pushed with `-q` and Fleet never moved it to Deliver).
 */
export function pushIn(command: string, output: string): { branch?: string } | undefined {
  const push = /\bgit\b[^|;&\n]*?\bpush\b([^|;&\n]*)/.exec(command)
  if (push === null || /--dry-run|(?:^|\s)-n(?:\s|$)/.test(command)) return undefined
  const sent = /^\s*(?:[0-9a-f]{7,40}\.\.\.?[0-9a-f]{7,40}|\*\s+\[new (?:branch|tag)\]|\+\s+[0-9a-f]{7,40}\.\.\.[0-9a-f]{7,40})\s+\S+\s+->\s+(\S+)/m.exec(output)
  if (sent !== null) return { branch: sent[1] }
  const words = (push[1] ?? '').trim().split(/\s+/).filter(word => word !== '')
  if (!words.some(word => word === '-q' || word === '--quiet')) return undefined
  const named = words.filter(word => !word.startsWith('-')).slice(1).at(-1)
  const branch = named?.replace(/^\+/, '').split(':').at(-1)?.replace(/^refs\/heads\//, '')
  return branch === undefined || branch === '' || branch === 'HEAD' ? {} : { branch }
}

/**
 * Whether a push is the session's own: it names no branch, the session's is not known, or they are the
 * same. A push of another branch, such as one into a scratch repository, moves no Job (28 Oct 2026).
 */
export function isOwnPush(pushed: { branch?: string }, own: string | undefined): boolean {
  return pushed.branch === undefined || own === undefined || pushed.branch === own
}

/** One item of a session's `waiting_for` list, as the agent stated it (`WaitingInput`, `crates/ipc/src/sessions.rs`). */
export type WaitingItem = { id: string; text: string; act?: unknown; options?: unknown; steps?: unknown }

const NEED_MOST = 200
const NEEDS_MOST = 10
const FENCE = /^\s*(```|~~~)/
const HEADING = /^\s{0,3}#{1,6}\s+(.*?)\s*#*\s*$/
const BOLD_LINE = /^\s*\*\*([^*]+?)\*\*:?\s*$/
const DIVIDER = /^\s*\|?\s*:?-{2,}:?\s*(\|\s*:?-{2,}:?\s*)*\|?\s*$/
const LIST_ITEM = /^\s*(?:[-*+]|\d+[.)])\s+(.*)$/
const OWNER_HEAD = /^(owner|owned by)$/
// Read as an owner column only under an Actions heading, where the table is the owner's list of acts.
const WHO_HEAD = /^(who|assignee|assigned to)$/
const STATUS_HEAD = /^(status|state)$/
const ACT_HEAD = /^(action|what|item|task|step|next step|to ?do|ask|need)s?$/
const INDEX_HEAD = /^(#|no\.?|n)?$/
// What owns a row that asks nothing of a person. `me`, `we` and `us` are the agent writing; a
// kebab-case owner is a subagent type (`rust-engineer`).
const NOT_A_PERSON = /\b(agent|agents|subagents?|claude|assistant|ai|bot|ci|fleet|drone|judge|helm|armada|bridge|github|automation|system|explore|plan|reviewer|engineer|me|we|us)\b|\w-\w|^(-|—|–|n\/a|none|tbd)?$/
const DONE = /^(✅|✔|✓|☑)|\b(done|completed?|merged|closed|resolved|shipped|landed|finished)\b/
const NOT_DONE = /\b(not|pending|blocked|waiting|open|todo|to do)\b|❌|⏳/
const NO_QUESTIONS = /^(none|nothing|no (open )?questions?)\b/
// The marker is upper case, or bold: a lower-case `question:` is prose.
const QUESTION_LINE = /^\s*(?:[-*+]\s+)?QUESTION:\s*(.+)$/
const BOLD_QUESTION = /^\s*(?:[-*+]\s+)?\*\*question:\*\*\s*(.+)$/i

function plain(text: string): string {
  return text.replace(/[*_`~]/g, '').replace(/\[([^\]]*)\]\([^)]*\)/g, '$1').replace(/\s+/g, ' ').trim()
}

function needLine(text: string): string {
  const flat = plain(text)
  return flat.length > NEED_MOST ? `${flat.slice(0, NEED_MOST - 1)}…` : flat
}

function cellsOf(row: string): string[] {
  const inner = row.trim().replace(/^\|/, '').replace(/\|$/, '')
  return inner.split(/(?<!\\)\|/).map(cell => cell.replace(/\\\|/g, '|').trim())
}

function isPerson(owner: string): boolean {
  return plain(owner)
    .toLowerCase()
    .split(/\s*(?:\/|,|&|\+|\band\b)\s*/)
    .some(part => part !== '' && !NOT_A_PERSON.test(part))
}

function isDone(status: string): boolean {
  const said = plain(status).toLowerCase()
  return DONE.test(said) && !NOT_DONE.test(said)
}

/** The rows of one table a person owns and has not done, each as the act it names. */
function tableNeeds(rows: string[], underActions: boolean): string[] {
  const head = cellsOf(rows[0] ?? '').map(cell => plain(cell).toLowerCase())
  const owner = head.findIndex(cell => OWNER_HEAD.test(cell) || (underActions && WHO_HEAD.test(cell)))
  if (owner < 0) return []
  const status = head.findIndex(cell => STATUS_HEAD.test(cell))
  const named = head.findIndex(cell => ACT_HEAD.test(cell))
  const found: string[] = []
  for (const row of rows.slice(2)) {
    const cells = cellsOf(row)
    if (cells.every(cell => cell === '') || /^\s*~~/.test(cells.join(''))) continue
    if (!isPerson(cells[owner] ?? '')) continue
    if (status >= 0 && isDone(cells[status] ?? '')) continue
    const act =
      named >= 0
        ? cells[named]
        : cells.find((cell, at) => at !== owner && at !== status && !INDEX_HEAD.test(head[at] ?? '') && plain(cell) !== '')
    if (act !== undefined && plain(act) !== '') found.push(needLine(act))
  }
  return found
}

/**
 * What a reply says the person owes, one line each: a row a person owns in a table with an Owner
 * column (an Actions table may name it Who), that is not marked done; an item under a Questions
 * heading; a `QUESTION:` line. **Conservative**, as a line found sends the session back for a turn:
 * a fenced block is never read, and a table with no owner column names nobody.
 */
export function needsSaidIn(answer: string): string[] {
  const lines = answer.split('\n')
  const found: string[] = []
  let fenced = false
  let section = ''
  for (let at = 0; at < lines.length; at += 1) {
    const line = lines[at] ?? ''
    if (FENCE.test(line)) {
      fenced = !fenced
      continue
    }
    if (fenced) continue
    const heading = HEADING.exec(line) ?? BOLD_LINE.exec(line)
    if (heading !== null) {
      section = plain(heading[1] ?? '').replace(/:$/, '').toLowerCase()
      continue
    }
    const asked = QUESTION_LINE.exec(line) ?? BOLD_QUESTION.exec(line)
    if (asked !== null) {
      found.push(needLine(asked[1] ?? ''))
      continue
    }
    if (line.trim().startsWith('|') && DIVIDER.test(lines[at + 1] ?? '')) {
      const rows: string[] = []
      while ((lines[at] ?? '').trim().startsWith('|')) rows.push(lines[at++] ?? '')
      at -= 1
      found.push(...tableNeeds(rows, /\bactions?\b/.test(section)))
      continue
    }
    if (/^(open )?questions?\b/.test(section)) {
      const item = LIST_ITEM.exec(line)
      const text = item?.[1] ?? (line.trim().endsWith('?') ? line : undefined)
      if (text !== undefined && !NO_QUESTIONS.test(plain(text).toLowerCase())) found.push(needLine(text))
    }
  }
  return [...new Set(found)].slice(0, NEEDS_MOST)
}

/** The list a `waiting_for` call set, from the tool's input (`{ body: { session_id, waiting_for } }`). */
export function waitingItemsOf(input: Record<string, unknown>): WaitingItem[] | undefined {
  let body: unknown = input.body ?? input
  if (typeof body === 'string') {
    try {
      body = JSON.parse(body)
    } catch {
      return undefined
    }
  }
  const list = (body as { waiting_for?: unknown; items?: unknown } | null)?.waiting_for ?? (body as { items?: unknown } | null)?.items
  if (!Array.isArray(list)) return undefined
  return list.filter(
    (item): item is WaitingItem => typeof item?.id === 'string' && typeof item?.text === 'string',
  )
}

function keyWords(text: string): Set<string> {
  return new Set(
    plain(text)
      .toLowerCase()
      .split(/[^a-z0-9#]+/)
      .filter(word => word.length >= 4 || /\d/.test(word)),
  )
}

/**
 * Whether the list holds each line: an item shares two of its words, or all where it has fewer. Loose
 * on purpose, as an item is the line said shorter and a miss costs the session a turn.
 */
export function isCovered(needs: string[], items: WaitingItem[]): boolean {
  const held = items.map(item => keyWords(item.text))
  return needs.every(need => {
    const words = [...keyWords(need)]
    const enough = Math.min(2, words.length)
    return enough === 0 || held.some(item => words.filter(word => item.has(word)).length >= enough)
  })
}

/** What sends a session back to put what its reply named in `waiting_for`. Read by the agent. */
export function sentBackReason(needs: string[], sessionId: string): string {
  return [
    'Your reply names what the owner owes, and the armada waiting_for list does not hold it:',
    ...needs.map(need => `- ${need}`),
    `Put each in waiting_for with its own id, one short line and an act, passing session_id ${sessionId}, then end the turn. Do not repeat them as a table.`,
  ].join('\n')
}

const FROM_REPLY = 'From the reply: '

/** The list with what the reply named and the agent left out, each marked as read from the reply. */
export function withReplyNeeds(items: WaitingItem[], needs: string[]): WaitingItem[] {
  const missing = needs.filter(need => !isCovered([need], items))
  return [...items, ...missing.map((need, at) => ({ id: `reply:${at + 1}`, text: `${FROM_REPLY}${need}` }))]
}

/**
 * The agent's own items in the list Fleet holds, from `GET /sessions/one` (`SessionThread`). Items Fleet
 * raised itself (a walk, a question card, a permission) are left out: Fleet keeps those apart and would
 * read them posted back as the agent's. Nothing where the answer is not a session.
 */
export function agentItemsIn(thread: string): WaitingItem[] | undefined {
  let read: unknown
  try {
    read = JSON.parse(thread)
  } catch {
    return undefined
  }
  const list = (read as { session?: { waiting_for?: unknown } } | null)?.session?.waiting_for
  if (!Array.isArray(list)) return undefined
  return list
    .filter(item => item?.source === 'agent' && typeof item.id === 'string' && typeof item.text === 'string')
    .map(({ id, text, act, options, steps }) => ({
      id,
      text,
      ...(act === undefined ? {} : { act }),
      ...(options === undefined ? {} : { options }),
      ...(steps === undefined ? {} : { steps }),
    }))
}

// The Loadout: what a session's agent knows, was told and used, each with where it came from.
// `crates/ipc/src/sessions.rs`. **Names, sources and counts only**, never a body or a text.

/** Where an item came from. `kit` is the person's harness home, `~/.claude`; `managed` an organisation's settings. */
export type LoadoutSource = 'kit' | 'armada' | 'repository' | 'plugin' | 'mcp' | 'built_in' | 'managed'

export type LoadoutKind = 'skill' | 'subagent' | 'mcp_server' | 'tool' | 'memory_file' | 'output_style' | 'rules' | 'injection'

export type LoadoutItem = {
  kind: LoadoutKind
  name: string
  source: LoadoutSource
  from?: string
  state?: 'connected' | 'failed' | 'pending' | 'deferred' | 'loaded'
  tokens?: number
}

export type LoadoutUse = {
  turn_started_at: string
  kind: 'skill' | 'tool' | 'subagent'
  name: string
  source: LoadoutSource
  from?: string
  model?: string
  by?: string
}

type Origin = { source: LoadoutSource; from?: string }

export type ServerState = 'connected' | 'failed' | 'pending'

/** This mod's own plugin name, so what it ships reads as Armada's and not as any plugin's. */
const ARMADA_PLUGIN = 'armada'
/** The harness's own output styles; `default` is no style at all and is not an item. */
const BUILT_IN_STYLES = ['Explanatory', 'Learning']
/** The name Armada's own rules go by, as Kit lists them (`crates/fleet/src/kit.rs`). */
export const ARMADA_RULES = 'Armada session rules'
/** The block `prompt.context` adds with the session id (`idNote`). */
export const ARMADA_INJECTION = 'armadaSession'

function pluginOrigin(plugin: string | undefined): Origin | undefined {
  if (plugin === undefined || plugin === '') return undefined
  return plugin === ARMADA_PLUGIN ? { source: 'armada' } : { source: 'plugin', from: plugin }
}

/** A plugin's skill, agent or style is spelled `<plugin>:<name>`. */
function prefixOf(name: string): string | undefined {
  const at = name.indexOf(':')
  return at > 0 ? name.slice(0, at) : undefined
}

/** A skill's source by the engine's word for it (`SkillSource`); nothing for one Armada has no word for. */
export function skillOrigin(source: string, name: string, plugin?: string): Origin | undefined {
  if (source === 'userSettings' || source === 'syncedSkills') return { source: 'kit' }
  if (source === 'projectSettings') return { source: 'repository' }
  if (source === 'policySettings') return { source: 'managed' }
  if (source === 'plugin') return pluginOrigin(plugin ?? prefixOf(name))
  if (source === 'built-in' || source === 'bundled') return { source: 'built_in' }
  if (source === 'mcp') {
    const server = prefixOf(name)
    return server === undefined ? undefined : { source: 'mcp', from: server }
  }
  return undefined
}

/** A subagent's source by the engine's word for where it was defined (`ContextAgent.source`). */
export function agentOrigin(source: string, type: string): Origin | undefined {
  if (source === 'userSettings') return { source: 'kit' }
  if (source === 'projectSettings' || source === 'localSettings') return { source: 'repository' }
  if (source === 'policySettings' || source === 'flagSettings') return { source: 'managed' }
  if (source === 'plugin') return pluginOrigin(prefixOf(type))
  if (source === 'built-in') return { source: 'built_in' }
  return undefined
}

/**
 * A memory file's source and the name it goes by: under `~/.claude` the person's, under the session's
 * root the repository's, relative to it. Elsewhere the engine's label decides.
 */
export function memoryItem(path: string, type: string, home: string, root: string): { name: string; source: LoadoutSource } | undefined {
  const kit = `${home.replace(/\/$/, '')}/.claude/`
  const repository = `${root.replace(/\/$/, '')}/`
  if (path.startsWith(kit)) return { name: `~/.claude/${path.slice(kit.length)}`, source: 'kit' }
  if (path.startsWith(repository)) return { name: path.slice(repository.length), source: 'repository' }
  if (type === 'Project' || type === 'Local') return { name: path, source: 'repository' }
  if (type === 'User' || type === 'AutoMem') return { name: path, source: 'kit' }
  if (type === 'Managed') return { name: path, source: 'managed' }
  return undefined
}

/**
 * An MCP server's name and source. A plugin's server is listed as `plugin:<plugin>:<server>` and is
 * that plugin's; any other is its own.
 */
export function serverItem(listed: string): { name: string; source: LoadoutSource; from: string } {
  const parts = /^plugin:([^:]+):(.+)$/.exec(listed)
  if (parts === null) return { name: listed, source: 'mcp', from: listed }
  const [, plugin = '', name = ''] = parts
  return { name, source: plugin === ARMADA_PLUGIN ? 'armada' : 'plugin', from: plugin }
}

/** The server an `mcp__<server>__<tool>` name names, as the tool name spells it. */
export function serverOfTool(tool: string): string | undefined {
  const parts = /^mcp__(.+?)__.+$/.exec(tool)
  return parts?.[1]
}

/** A tool's source: an MCP tool's server as `/mcp` lists it where the breakdown says, else the harness's. */
export function toolOrigin(tool: string, mcpTools: readonly { name: string; serverName: string }[] = []): Origin {
  const server = mcpTools.find(one => one.name === tool)?.serverName ?? serverOfTool(tool)
  return server === undefined ? { source: 'built_in' } : { source: 'mcp', from: server }
}

/**
 * The output style's source. A plugin's is `<plugin>:<name>`; Armada's own is `Armada`; a style
 * found in the repository's `.claude/output-styles` is the repository's, and any other the person's.
 */
export function styleOrigin(name: string, inRepository: boolean): Origin {
  if (name === 'Armada') return { source: 'armada' }
  const plugin = pluginOrigin(prefixOf(name))
  if (plugin !== undefined) return plugin
  if (BUILT_IN_STYLES.includes(name)) return { source: 'built_in' }
  return { source: inRepository ? 'repository' : 'kit' }
}

/** What the mod holds of a session to make its Loadout from. */
export type Equipment = {
  breakdown?: SessionContextBreakdown
  home: string
  root: string
  /** The output style in force and its source; absent for the default. */
  style?: { name: string } & Origin
  /** Whether Fleet hosts the session, so Armada's session rules ride on its prompt (`ARMADA_SESSION`). */
  rules: boolean
  /** What `ToolSearch` and `WaitForMcpServers` last said of each server. */
  servers: Record<string, ServerState>
  /** The tools a `ToolSearch` loaded. */
  loaded: string[]
}

/**
 * The whole Loadout, in a fixed order so two readings that hold the same compare equal. An item the
 * engine gives no source Armada has a word for is left out rather than guessed.
 */
export function loadoutOf(equipment: Equipment): LoadoutItem[] {
  const { breakdown, home, root } = equipment
  const items: LoadoutItem[] = []
  const tokens = (count: number) => (Number.isFinite(count) && count > 0 ? { tokens: count } : {})
  for (const skill of breakdown?.skills?.skillFrontmatter ?? []) {
    const origin = skillOrigin(skill.source, skill.name, skill.pluginName)
    if (origin !== undefined) items.push({ kind: 'skill', name: skill.name, ...origin, ...tokens(skill.tokens) })
  }
  for (const agent of breakdown?.agents ?? []) {
    const origin = agentOrigin(agent.source, agent.agentType)
    if (origin !== undefined) items.push({ kind: 'subagent', name: agent.agentType, ...origin, ...tokens(agent.tokens) })
  }
  const mcpTools = breakdown?.mcpTools ?? []
  const servers = new Map<string, ServerState>(Object.entries(equipment.servers))
  // A server whose tools the window lists is connected, whatever was said of it before.
  for (const tool of mcpTools) servers.set(tool.serverName, 'connected')
  for (const [name, state] of [...servers].sort(([a], [b]) => a.localeCompare(b))) {
    items.push({ kind: 'mcp_server', ...serverItem(name), state })
  }
  const loaded = new Set(equipment.loaded)
  for (const tool of mcpTools) {
    const state = tool.isLoaded || loaded.has(tool.name) ? 'loaded' : 'deferred'
    items.push({ kind: 'tool', name: tool.name, source: 'mcp', from: tool.serverName, state, ...tokens(tool.tokens) })
  }
  for (const name of [...loaded].sort()) {
    if (mcpTools.some(tool => tool.name === name)) continue
    items.push({ kind: 'tool', name, ...toolOrigin(name), state: 'loaded' })
  }
  for (const file of breakdown?.memoryFiles ?? []) {
    const named = memoryItem(file.path, file.type, home, root)
    if (named !== undefined) items.push({ kind: 'memory_file', ...named, ...tokens(file.tokens) })
  }
  if (equipment.style !== undefined) {
    const { name, ...origin } = equipment.style
    items.push({ kind: 'output_style', name, ...origin })
  }
  if (equipment.rules) items.push({ kind: 'rules', name: ARMADA_RULES, source: 'armada' })
  items.push({ kind: 'injection', name: ARMADA_INJECTION, source: 'armada' })
  return items
}

/** What a `ToolSearch` result says: the tools it loaded, and the servers not yet or never connected. */
export function searchedOf(result: unknown): { matches: string[]; servers: Record<string, ServerState> } {
  const read = (result ?? {}) as { matches?: unknown; pending_mcp_servers?: unknown; failed_mcp_servers?: unknown }
  const names = (list: unknown) => (Array.isArray(list) ? list.filter((one): one is string => typeof one === 'string') : [])
  const servers: Record<string, ServerState> = {}
  for (const name of names(read.pending_mcp_servers)) servers[name] = 'pending'
  if (Array.isArray(read.failed_mcp_servers)) {
    for (const one of read.failed_mcp_servers) if (typeof one?.name === 'string') servers[one.name] = 'failed'
  }
  return { matches: names(read.matches), servers }
}

/** What a `WaitForMcpServers` result says of each server. One that needs signing in to gives no tools, so it is `failed`. */
export function waitedOf(result: unknown): Record<string, ServerState> {
  const read = (result ?? {}) as Record<string, unknown>
  const servers: Record<string, ServerState> = {}
  const mark = (key: string, state: ServerState) => {
    const list = read[key]
    if (Array.isArray(list)) for (const name of list) if (typeof name === 'string') servers[name] = state
  }
  mark('stillPending', 'pending')
  mark('connected', 'connected')
  mark('failed', 'failed')
  mark('needsAuth', 'failed')
  return servers
}

/** A skill the agent called, sourced from the listing the window carries, else by its plugin prefix. */
export function skillUseOrigin(name: string, breakdown?: SessionContextBreakdown): Origin | undefined {
  const bare = name.replace(/^\//, '')
  const listed = breakdown?.skills?.skillFrontmatter.find(one => one.name === bare)
  if (listed !== undefined) return skillOrigin(listed.source, listed.name, listed.pluginName)
  return pluginOrigin(prefixOf(bare))
}

/**
 * A subagent dispatched, sourced from the agent listing the window carries. The listing leaves the
 * harness's own out, so one it does not name that the engine provides is built in.
 */
export function agentUseOrigin(type: string, provider: string, breakdown?: SessionContextBreakdown): Origin | undefined {
  const listed = breakdown?.agents.find(one => one.agentType === type)
  if (listed !== undefined) return agentOrigin(listed.source, listed.agentType)
  if (provider !== 'engine') return pluginOrigin(provider)
  return prefixOf(type) === undefined ? { source: 'built_in' } : pluginOrigin(prefixOf(type))
}
