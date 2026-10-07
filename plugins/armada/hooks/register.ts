// Tells a running Fleet which sessions this machine has open and what each holds.
// `docs/concepts/session.md`.
//
// **A session is reported as it happens and never read back**, with one
// exception: a message a person sent it from Bridge, which Fleet holds until
// this mod asks (`submitHeld`). No hook changes what a session does or is told:
// each answers with what `next` returned and lets its report go unawaited, so
// Fleet being down costs the session nothing (`fleet.ts`). **No message text and no prompt leaves, apart from the first
// line of the first prompt as a title.** A message is reported as who it went to
// or came from, and how many.
//
// The helpers are top-level because the engine follows `$` only into a function
// declared at the top of a file, and refuses the module otherwise.

import type { Engine, Register } from 'claude-code'

import {
  customTitleIn,
  ghAct,
  isDispatch,
  jobIdsIn,
  mayMoveBranch,
  needAct,
  micros,
  pullRequestsIn,
  senderOf,
  titleOf,
  effortOf,
  modeOf,
  modelName,
  transcriptPath,
} from './facts'
import type { Door, Fact, Report } from './fleet'

const HARNESS = 'claude_code'
const MEASURE_EVERY_MS = 10_000
const RUNTIME_FILE = 'Library/Application Support/Armada/fleet.json'
const WAIT_MS = 1500
const ASK_EVERY_MS = 2000
const SILENT_MS = 30_000
const DISPATCHES = ['propose_job', 'propose_from_request', 'approve_dispatch', 'redispatch_job']

type Dollar = Door & Pick<Engine, 'command' | 'process' | 'prompt' | 'session'>

type Known = {
  cwd: string
  title?: string
  branch?: string
  prs: Map<string, string>
  needs: Map<string, Record<string, string>>
  messages: Map<string, number>
  tuned?: Tuning
}

type Tuning = Extract<Fact, { kind: 'tuned' }>

const known = new Map<string, Known>()
let measuredAt = Number.NEGATIVE_INFINITY

function told(id: string, fact: Fact): Report {
  return { harness: HARNESS, session_id: id, fact }
}

function everything(): Report[] {
  return [...known].flatMap(([id, one]) => [
    told(id, { kind: 'started', cwd: one.cwd, title: one.title, origin: 'terminal' }),
    ...(one.tuned === undefined ? [] : [told(id, one.tuned)]),
    ...(one.branch === undefined
      ? []
      : [told(id, { kind: 'attached', attachment: { kind: 'branch', target: one.branch } })]),
    ...[...one.needs].map(([path, detail]) =>
      told(id, { kind: 'attached', attachment: { kind: 'need', target: path, detail } }),
    ),
    ...[...one.prs].map(([number, url]) =>
      told(id, { kind: 'attached', attachment: { kind: 'pr', target: number, detail: { url } } }),
    ),
  ])
}

// What was dropped while Fleet was out of reach is not replayed, so what the
// sessions hold is told again from `everything` when it answers.
let queue: Promise<void> = Promise.resolve()
let silentUntil = 0
let wasOut = false


async function portOf($: Door): Promise<number | undefined> {
  const home = await $.env.get('HOME')
  if (!home) return undefined
  const file = JSON.parse(await $.fs.read(`${home}/${RUNTIME_FILE}`)) as { port?: unknown }
  const port = file.port
  if (typeof port !== 'number' || !Number.isInteger(port) || port < 1 || port > 65535) {
    return undefined
  }
  return port
}

async function post($: Door, report: Report): Promise<boolean> {
  const port = await portOf($)
  if (port === undefined) return false
  const sent = $.http.fetch(`http://127.0.0.1:${port}/sessions/report`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(report),
  })
  const answered = await Promise.race([sent, $.clock.sleep(WAIT_MS).then(() => undefined)])
  // A refusal is Fleet answering; only silence is Fleet being out of reach.
  return answered !== undefined
}

async function deliver($: Door, report: Report): Promise<void> {
  const now = await $.clock.now()
  if (now < silentUntil) return
  try {
    const reports = wasOut ? [...everything(), report] : [report]
    for (const one of reports) {
      if (!(await post($, one))) throw new Error('out of reach')
    }
    wasOut = false
  } catch {
    silentUntil = now + SILENT_MS
    wasOut = true
  }
}

// What a person sent this session from Bridge. Each is submitted as the
// person's own prompt, which starts a turn when the session is idle and waits
// for it when it is not (spike 27). One at a time, so they keep their order.
let submitting: Promise<void> = Promise.resolve()

async function submitHeld($: Dollar): Promise<void> {
  if ((await $.clock.now()) < silentUntil) return
  const port = await portOf($)
  if (port === undefined) return
  const id = await $.session.id()
  const asked = await $.http.fetch(`http://127.0.0.1:${port}/sessions/held`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ session_id: id }),
  })
  if (!asked.ok) return
  const held = JSON.parse(asked.text) as { messages?: unknown; commands?: unknown }
  // A command is run as typed: the engine refuses a slash command submitted as text.
  const commands = Array.isArray(held.commands) ? held.commands : []
  for (const one of commands as { command?: unknown; args?: unknown }[]) {
    if (typeof one.command !== 'string' || typeof one.args !== 'string') continue
    const { command, args } = one
    submitting = submitting
      .then(() => $.command.run({ command, args }))
      .then(() => tuned($, id, {}))
      .catch(() => undefined)
  }
  if (!Array.isArray(held.messages)) return
  for (const text of held.messages) {
    if (typeof text !== 'string') continue
    submitting = submitting
      .then(() => $.prompt.submit({ text, asUser: true }))
      .then(() => undefined)
      .catch(() => undefined)
  }
}

/** Queue a report, in order. Never throws and never waits. */
function send($: Door, report: Report): void {
  queue = queue.then(() => deliver($, report)).catch(() => undefined)
}

/** Wait, within a short bound, for what is queued. */
async function flush($: Door): Promise<void> {
  await Promise.race([queue, $.clock.sleep(WAIT_MS * 2)])
}

// A fact that was still being worked out when its session ended is dropped, so
// `ended` is the last thing Fleet hears of a session.
function say($: Door, id: string, fact: Fact): void {
  if (fact.kind !== 'started' && fact.kind !== 'ended' && !known.has(id)) return
  send($, told(id, fact))
}

function settle($: Door, id: string, kind: string, target: string, state: 'spent' | 'given_back') {
  say($, id, { kind: 'settled', attachment: { kind, target }, state })
}

async function branchOf($: Dollar, cwd: string): Promise<string | undefined> {
  const ran = await $.process.run(['git', 'rev-parse', '--abbrev-ref', 'HEAD'], {
    cwd,
    timeoutMs: 3000,
  })
  const name = ran.stdout.trim()
  return ran.exitCode === 0 && name !== '' && name !== 'HEAD' ? name : undefined
}

async function pullRequestOn($: Dollar, id: string, one: Known): Promise<void> {
  const ran = await $.process.run(['gh', 'pr', 'view', '--json', 'number,url,state'], {
    cwd: one.cwd,
    timeoutMs: 10_000,
  })
  if (ran.exitCode !== 0) return
  const view = JSON.parse(ran.stdout) as { number?: number; url?: string; state?: string }
  if (typeof view.number !== 'number') return
  const number = String(view.number)
  const detail = { url: view.url ?? '', state: (view.state ?? '').toLowerCase() }
  one.prs.set(number, detail.url)
  say($, id, { kind: 'attached', attachment: { kind: 'pr', target: number, detail } })
  if (view.state === 'MERGED') settle($, id, 'pr', number, 'spent')
  if (view.state === 'CLOSED') settle($, id, 'pr', number, 'given_back')
}

/** Where the branch is now, and the pull request on it. */
async function look($: Dollar, id: string): Promise<void> {
  try {
    const one = known.get(id)
    if (one === undefined) return
    const branch = await branchOf($, one.cwd)
    if (branch === one.branch) return
    if (one.branch !== undefined && branch === undefined) {
      settle($, id, 'branch', one.branch, 'given_back')
    }
    one.branch = branch
    if (branch === undefined) return
    say($, id, { kind: 'attached', attachment: { kind: 'branch', target: branch } })
    await pullRequestOn($, id, one).catch(() => undefined)
  } catch {
    // A directory with no repository, or no `git`: nothing to report.
  }
}

async function begin($: Dollar, id: string, cwd?: string): Promise<void> {
  const where = cwd ?? (await $.session.cwd())
  known.set(id, { cwd: where, prs: new Map(), needs: new Map(), messages: new Map() })
  say($, id, { kind: 'started', cwd: where, origin: 'terminal' })
  void look($, id)
  void tuned($, id, {}, true).catch(() => undefined)
}

/**
 * What the terminal runs on, told when it changed: the model, and the effort and permission mode a
 * hook input carries. The commands it lists go once, with the first. **The mode is only read**: the
 * mods API cannot switch a live session's (spike 27).
 */
async function tuned(
  $: Dollar,
  id: string,
  seen: { mode?: string; effort?: unknown },
  withCommands = false,
): Promise<void> {
  const one = known.get(id)
  if (one === undefined) return
  const mode = modeOf(seen.mode)
  const effort = effortOf(seen.effort)
  const next: Tuning = {
    kind: 'tuned',
    model: modelName(await $.session.model()),
    ...(effort === undefined ? {} : { effort }),
    ...(mode === undefined ? {} : { mode }),
  }
  if (withCommands) {
    next.commands = (await $.command.list()).map(c => ({ name: c.name, says: c.description }))
  }
  const before = one.tuned
  const same =
    before !== undefined &&
    before.model === next.model &&
    before.effort === (next.effort ?? before.effort) &&
    before.mode === (next.mode ?? before.mode)
  if (same && !withCommands) return
  one.tuned = { ...before, ...next, commands: undefined }
  say($, id, next)
}

async function seen($: Dollar, mode: string | undefined, effort: unknown): Promise<void> {
  try {
    const [id] = await current($)
    await tuned($, id, { mode, effort })
  } catch {
    // What the terminal runs on is a nicety: nothing here is worth a session's turn.
  }
}

/** The session's id, starting it first where it began without a `session.start`. */
async function current($: Dollar): Promise<[string, Known]> {
  const id = await $.session.id()
  if (!known.has(id)) await begin($, id)
  return [id, known.get(id) as Known]
}

async function afterBash($: Dollar, command: string, text: string): Promise<void> {
  const [id, one] = await current($)
  const need = needAct(command)
  if (need?.act === 'declare') {
    const detail = { what: need.what }
    one.needs.set(need.path, detail)
    say($, id, { kind: 'attached', attachment: { kind: 'need', target: need.path, detail } })
  }
  // Without the words it was declared in, a took would overwrite them with nothing.
  const held = need?.act === 'took' ? one.needs.get(need.path) : undefined
  if (need?.act === 'took' && held !== undefined) {
    const detail = { what: held.what, took: need.value }
    one.needs.set(need.path, detail)
    say($, id, { kind: 'attached', attachment: { kind: 'need', target: need.path, detail } })
  }
  if (need?.act === 'release') {
    one.needs.delete(need.path)
    settle($, id, 'need', need.path, 'given_back')
  }
  const act = ghAct(command)
  if (act?.act === 'create') {
    for (const pr of pullRequestsIn(text)) {
      one.prs.set(pr.number, pr.url)
      say($, id, {
        kind: 'attached',
        attachment: { kind: 'pr', target: pr.number, detail: { url: pr.url } },
      })
    }
  }
  if (act?.act === 'merge' || act?.act === 'close') {
    const target = act.number ?? (one.prs.size === 1 ? [...one.prs.keys()][0] : undefined)
    if (target !== undefined) {
      settle($, id, 'pr', target, act.act === 'merge' ? 'spent' : 'given_back')
    }
  }
  if (mayMoveBranch(command) || act?.act === 'create') {
    const cwd = await $.session.cwd()
    if (cwd !== one.cwd) {
      one.cwd = cwd
      say($, id, { kind: 'moved', cwd })
    }
    await look($, id)
  }
}

async function dispatched($: Dollar, via: string, text: string): Promise<void> {
  const [id] = await current($)
  for (const job of jobIdsIn(text)) {
    say($, id, { kind: 'attached', attachment: { kind: 'job', target: job, detail: { via } } })
  }
}

async function spawned(
  $: Dollar,
  target: string,
  type: string,
  description: string,
): Promise<void> {
  const [id] = await current($)
  const detail = { type, description: description.slice(0, 80) }
  say($, id, { kind: 'attached', attachment: { kind: 'subagent', target, detail } })
}

async function messaged($: Dollar, direction: 'sent' | 'received', who: string): Promise<void> {
  const [id, one] = await current($)
  const target = `${direction === 'sent' ? 'to' : 'from'}:${who}`
  const count = (one.messages.get(target) ?? 0) + 1
  one.messages.set(target, count)
  const detail = { direction, count: String(count) }
  say($, id, { kind: 'attached', attachment: { kind: 'message', target, detail } })
}

async function measured($: Dollar, tokens?: number, window?: number, usd?: number): Promise<void> {
  const at = await $.clock.now()
  if (at - measuredAt < MEASURE_EVERY_MS) return
  measuredAt = at
  const [id] = await current($)
  const usage = { context_tokens: tokens, context_window: window, cost_micros: micros(usd) }
  say($, id, { kind: 'measured', usage })
}

async function titled($: Dollar, prompt: string): Promise<void> {
  const [id, one] = await current($)
  const title = one.title === undefined ? titleOf(prompt) : undefined
  if (title === undefined) return
  one.title = title
  say($, id, { kind: 'titled', title })
}

/**
 * A `/rename` in the terminal. **The name is what was typed**; a bare `/rename` has Claude Code
 * make one up, which is read from the transcript it writes the entry to. It replaces any title
 * and is told as `named`, so the first prompt's line never takes it back.
 */
async function renamed($: Dollar, typed: string): Promise<void> {
  const [id, one] = await current($)
  let title: string | undefined = typed.replace(/\s+/g, ' ').trim() || undefined
  if (title === undefined) {
    try {
      const home = await $.env.get('HOME')
      const cwd = await $.session.cwd()
      title = home ? customTitleIn(await $.fs.read(transcriptPath(home, cwd, id))) : undefined
    } catch {
      // A transcript too large to read, or not there: the name is not known.
    }
  }
  if (title === undefined) return
  one.title = title
  say($, id, { kind: 'titled', title, named: true })
}

async function ended($: Dollar, id: string, reason: string): Promise<void> {
  if (!known.has(id)) await begin($, id)
  say($, id, { kind: 'ended', reason })
  known.delete(id)
  await flush($)
}

export const register: Register = on => {
  on('session.start', async ($, e, next) => {
    const started = await next(e)
    $.clock.every(ASK_EVERY_MS, () => submitHeld($).catch(() => undefined))
    void $.session
      .id()
      .then(id => begin($, id, e.cwd))
      .catch(() => undefined)
    return started
  })

  on('prompt.submit', async ($, e, next) => {
    const out = await next(e)
    void titled($, e.text).catch(() => undefined)
    return out
  })

  on('command.run', { command: 'rename' }, async ($, e, next) => {
    const out = await next(e)
    void renamed($, e.args).catch(() => undefined)
    return out
  })

  on('tool.call', { tool: 'Bash' }, async ($, e, next) => {
    const ran = await next(e)
    if (ran.deny === undefined && ran.isError !== true) {
      void afterBash($, e.command, ran.text ?? '').catch(() => undefined)
    }
    return ran
  })

  for (const name of DISPATCHES) {
    on('tool.call', { tool: `mcp__armada-fleet__${name}` }, async ($, e, next) => {
      const ran = await next(e)
      if (isDispatch(e.tool) && ran.deny === undefined && ran.isError !== true) {
        void dispatched($, name, ran.text ?? '').catch(() => undefined)
      }
      return ran
    })
  }

  on('agent.spawn', async ($, e, next) => {
    const out = await next(e)
    if (out.deny === undefined) {
      const target = out.agentId ?? e.tool_use_id
      void spawned($, target, e.subagentType, e.description).catch(() => undefined)
    }
    return out
  })

  on('session.send', async ($, e, next) => {
    const out = await next(e)
    if (out.isDelivered) void messaged($, 'sent', e.to).catch(() => undefined)
    return out
  })

  on('session.receive', async ($, e, next) => {
    const out = await next(e)
    const from = senderOf(e.origin as { kind: string; teammate?: string })
    void messaged($, 'received', from).catch(() => undefined)
    return out
  })

  on('session.measure', async ($, e, next) => {
    const out = await next(e)
    void measured($, e.context.tokens, e.context.window, e.cost?.usd).catch(() => undefined)
    return out
  })

  // The terminal's permission mode and effort are on the settings-hook inputs and nowhere else.
  on('classic.UserPromptSubmit', async ($, e, next) => {
    const out = await next(e)
    void seen($, e.permission_mode, e.effort)
    return out
  })

  on('classic.Stop', async ($, e, next) => {
    const out = await next(e)
    void seen($, e.permission_mode, e.effort)
    return out
  })

  on('turn.complete', async ($, e, next) => {
    const out = await next(e)
    if (e.agentId === undefined) {
      void current($)
        .then(([id]) => say($, id, { kind: 'turn_completed' }))
        .catch(() => undefined)
    }
    return out
  })

  on('session.end', async ($, e, next) => {
    const out = await next(e)
    await ended($, e.sessionId, e.reason).catch(() => undefined)
    return out
  })
}
