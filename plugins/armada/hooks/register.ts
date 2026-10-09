// Tells a running Fleet which sessions this machine has open and what each holds.
// `docs/concepts/session.md`.
//
// **A session is reported as it happens and never read back**, with one
// exception: a message a person sent it from Bridge, which Fleet holds until
// this mod asks (`submitHeld`). One thing is told to the model: its own session id, so
// `show_window` can name it. Two hooks change what a session does, an `open` of a page and an
// `AskUserQuestion` Bridge answers first; every other answers with what `next` returned and lets its report go unawaited, so
// Fleet being down costs the session nothing (`fleet.ts`). **No message text and no prompt leaves, apart from the first
// line of the first prompt as a title, and a question put to Bridge.** A message is reported as who it went to
// or came from, and how many.
//
// The helpers are top-level because the engine follows `$` only into a function
// declared at the top of a file, and refuses the module otherwise.

import type { Engine, Register } from 'claude-code'

import {
  answersIn,
  artifactOf,
  customTitleIn,
  ghAct,
  isDispatch,
  jobIdsIn,
  mayMoveBranch,
  needAct,
  micros,
  pagesOpenedIn,
  pullRequestsIn,
  senderOf,
  titleOf,
  effortOf,
  modeOf,
  modelName,
  MOD_VERSION,
  transcriptPath,
} from './facts'
import type { Asked, Door, Fact, Report } from './fleet'

const HARNESS = 'claude_code'
const MEASURE_EVERY_MS = 10_000
const RUNTIME_FILE = 'Library/Application Support/Armada/fleet.json'
const WAIT_MS = 1500
const ASK_EVERY_MS = 2000
const SILENT_MS = 30_000
const DOCS_ACTS = ['create', 'batch', 'update']
const DISPATCHES = ['propose_job', 'propose_from_request', 'approve_dispatch', 'redispatch_job']

type Dollar = Door & Pick<Engine, 'command' | 'process' | 'prompt' | 'session'>

type Known = {
  cwd: string
  title?: string
  branch?: string
  prs: Map<string, Record<string, string>>
  needs: Map<string, Record<string, string>>
  messages: Map<string, number>
  artifacts: Map<string, Record<string, string>>
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
    told(id, { kind: 'started', cwd: one.cwd, title: one.title, origin: 'terminal', mod_version: MOD_VERSION }),
    ...(one.tuned === undefined ? [] : [told(id, one.tuned)]),
    ...(one.branch === undefined
      ? []
      : [told(id, { kind: 'attached', attachment: { kind: 'branch', target: one.branch } })]),
    ...[...one.needs].map(([path, detail]) =>
      told(id, { kind: 'attached', attachment: { kind: 'need', target: path, detail } }),
    ),
    ...[...one.prs].map(([number, detail]) =>
      told(id, { kind: 'attached', attachment: { kind: 'pr', target: number, detail } }),
    ),
    ...[...one.artifacts].map(([target, detail]) =>
      told(id, { kind: 'attached', attachment: { kind: 'artifact', target, detail } }),
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
  const ran = await $.process.run(['gh', 'pr', 'view', '--json', 'number,url,state,title,headRefName,isDraft'], {
    cwd: one.cwd,
    timeoutMs: 10_000,
  })
  if (ran.exitCode !== 0) return
  const view = JSON.parse(ran.stdout) as {
    number?: number
    url?: string
    state?: string
    title?: string
    headRefName?: string
    isDraft?: boolean
  }
  if (typeof view.number !== 'number') return
  const number = String(view.number)
  // The ledger names a pull request by its title, so a row told without one draws as a bare number.
  const detail: Record<string, string> = {
    url: view.url ?? '',
    address: view.url ?? '',
    state: view.isDraft === true && view.state === 'OPEN' ? 'draft' : (view.state ?? '').toLowerCase(),
    ...(view.title === undefined ? {} : { title: view.title }),
    ...(view.headRefName === undefined ? {} : { branch: view.headRefName }),
  }
  one.prs.set(number, detail)
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

const SHOWN_INSTEAD =
  "Shown in Bridge's window instead. Use the armada show_window tool to show the owner a page; never `open` it."

async function showPages($: Dollar, urls: string[]): Promise<void> {
  try {
    const port = await portOf($)
    if (port === undefined) return
    const session_id = await $.session.id()
    for (const url of urls) {
      await Promise.race([
        $.http.fetch(`http://127.0.0.1:${port}/sessions/window`, {
          method: 'POST',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify({ url, session_id }),
        }),
        $.clock.sleep(WAIT_MS),
      ])
    }
  } catch {
    // Fleet out of reach: the line is still not run.
  }
}

/** How long Fleet holds one poll, and how long the mod waits on a request before it asks again. */
const POLL_FETCH_MS = 35_000
const POLL_RETRY_MS = 3000

async function postAsk($: Door, port: number, body: unknown, bound: number): Promise<Asked | undefined> {
  const sent = await Promise.race([
    $.http.fetch(`http://127.0.0.1:${port}/sessions/ask/terminal`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(body),
    }),
    $.clock.sleep(bound).then(() => undefined),
  ])
  return sent?.ok === true ? (JSON.parse(sent.text) as Asked) : undefined
}

/**
 * The terminal's question, put to Bridge. **Posted once, then polled for**: Fleet keeps the question
 * and any answer from Bridge, so a request that times out, a dropped connection or a Fleet that
 * restarts costs one poll and never the question. Resolves with the answer, or nothing where Fleet
 * says the card is gone. A Fleet that is out of reach is waited out quietly; the terminal's own
 * prompt is all there is meanwhile. `over` ends the loop once the terminal's prompt has.
 */
async function putToBridge($: Door, id: string, questions: unknown, over: { done: boolean }): Promise<Asked | undefined> {
  try {
    let call: string | undefined
    while (call === undefined && !over.done) {
      const port = await portOf($).catch(() => undefined)
      const put = port === undefined ? undefined : await postAsk($, port, { kind: 'asks', session_id: id, input: { questions } }, POLL_FETCH_MS).catch(() => undefined)
      if (put?.outcome === 'asked') call = put.call
      else if (put !== undefined) return undefined
      else await $.clock.sleep(POLL_RETRY_MS)
    }
    while (call !== undefined && !over.done) {
      const port = await portOf($).catch(() => undefined)
      const polled = port === undefined ? undefined : await postAsk($, port, { kind: 'wait', session_id: id, call }, POLL_FETCH_MS).catch(() => undefined)
      if (polled === undefined) await $.clock.sleep(POLL_RETRY_MS)
      else if (polled.outcome !== 'waiting') return polled
    }
    return undefined
  } catch {
    return undefined
  }
}

/** The terminal's own prompt ended first, so Bridge's card closes. */
async function settledInTerminal($: Door, id: string, answered: boolean): Promise<void> {
  try {
    const port = await portOf($)
    if (port === undefined) return
    await Promise.race([
      $.http.fetch(`http://127.0.0.1:${port}/sessions/ask/terminal`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ kind: 'settled', session_id: id, answered }),
      }),
      $.clock.sleep(WAIT_MS),
    ])
  } catch {
    // Fleet out of reach: its card closes when its hold runs out.
  }
}

function idNote(id: string): string {
  return `Your Armada session id is ${id}. Pass it as session_id to the armada show_window tool. To show the owner a web page (a walk, a mock, a dev server), call show_window. Never run \`open\`. Keep the armada waiting_for tool current: whenever you need the owner for something (a decision, a page to look at, a pull request to approve, a command only he can run), put it in the list with one short line each and call waiting_for with the whole list, passing session_id. Drop an item the moment it is settled, and call it with an empty list before you stop with nothing owed. Do not wait to be asked what is outstanding.`
}

async function begin($: Dollar, id: string, cwd?: string): Promise<void> {
  // A Drone, a Judge call or a scout loads this mod too, since it reads the operator's user
  // settings. Fleet marks those launches, and they are not sessions to list.
  if (await $.env.get('ARMADA_DRONE')) return
  const where = cwd ?? (await $.session.cwd())
  known.set(id, { cwd: where, prs: new Map(), needs: new Map(), messages: new Map(), artifacts: new Map() })
  say($, id, { kind: 'started', cwd: where, origin: 'terminal', mod_version: MOD_VERSION })
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
    next.mod_version = MOD_VERSION
  }
  const before = one.tuned
  const same =
    before !== undefined &&
    before.model === next.model &&
    before.effort === (next.effort ?? before.effort) &&
    before.mode === (next.mode ?? before.mode)
  if (same && !withCommands) return
  one.tuned = { ...before, ...next, commands: undefined, mod_version: undefined }
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
  const one = known.get(id)
  if (one === undefined) throw new Error('unreported session')
  return [id, one]
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
      const detail = { url: pr.url }
      one.prs.set(pr.number, detail)
      say($, id, { kind: 'attached', attachment: { kind: 'pr', target: pr.number, detail } })
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

/**
 * A page published, a document written or a Claude Docs document made: an artifact on the ledger.
 * Only what a person would open. A code edit is not told.
 */
async function made(
  $: Dollar,
  tool: string,
  input: Record<string, unknown>,
  text: string,
  created: boolean,
): Promise<void> {
  const artifact = artifactOf(tool, input, text, created)
  if (artifact === undefined) return
  const [id, one] = await current($)
  const detail: Record<string, string> = { form: artifact.form }
  if (artifact.title !== undefined) detail.title = artifact.title
  if (artifact.form === 'image' && one.artifacts.has(artifact.target)) return
  one.artifacts.set(artifact.target, detail)
  say($, id, { kind: 'attached', attachment: { kind: 'artifact', target: artifact.target, detail } })
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
    if (await $.env.get('ARMADA_DRONE')) return started
    $.clock.every(ASK_EVERY_MS, () => submitHeld($).catch(() => undefined))
    void $.session
      .id()
      .then(id => begin($, id, e.cwd))
      .catch(() => undefined)
    return started
  })

  // `show_window` places a terminal session by the id it names, which the model cannot learn otherwise.
  on('prompt.context', async ($, e, next) => {
    const out = await next(e)
    if (await $.env.get('ARMADA_DRONE')) return out
    const id = await $.session.id()
    return { ...out, blocks: [...out.blocks, { name: 'armadaSession', text: idNote(id) }] }
  })

  // Again at every start (startup, resume, clear, compact), so the id is never out of view.
  on('classic.SessionStart', async ($, e, next) => {
    const out = await next(e)
    if (await $.env.get('ARMADA_DRONE')) return out
    const id = await $.session.id()
    return { ...out, additionalContext: [...(out.additionalContext ?? []), idNote(id)] }
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

  // A page is shown in Bridge's window and lands on the ledger; `open` would go to the owner's browser.
  on('tool.call', { tool: 'Bash' }, async ($, e, next) => {
    const pages = pagesOpenedIn(e.command)
    if (pages.length > 0 && !(await $.env.get('ARMADA_DRONE'))) {
      await showPages($, pages)
      return { deny: SHOWN_INSTEAD }
    }
    const ran = await next(e)
    if (ran.deny === undefined && ran.isError !== true) {
      void afterBash($, e.command, ran.text ?? '').catch(() => undefined)
    }
    return ran
  })

  // A question asked in the terminal is also put to Bridge, and the first answer wins. The terminal's
  // prompt opens beneath (`next`) while Fleet holds the question; returning before `next` does aborts
  // the prompt, so an answer from Bridge is the tool's own result and the terminal never keeps it.
  on('tool.call', { tool: 'AskUserQuestion' }, async ($, e, next) => {
    if (await $.env.get('ARMADA_DRONE')) return next(e)
    const id = await $.session.id()
    const over = { done: false }
    const terminal = next(e).then(ran => ({ ran }))
    const bridge = putToBridge($, id, e.questions, over).then(asked =>
      asked?.outcome === 'answered' || asked?.outcome === 'refused'
        ? { asked }
        : new Promise<never>(() => undefined),
    )
    try {
      const first = await Promise.race([terminal, bridge])
      if ('asked' in first) {
        if (first.asked.outcome === 'refused') return { deny: first.asked.message }
        return { result: { questions: e.questions, answers: answersIn(first.asked.updated_input) } }
      }
      void settledInTerminal($, id, first.ran.deny === undefined && first.ran.isError !== true)
      return first.ran
    } catch (error) {
      void settledInTerminal($, id, false)
      throw error
    } finally {
      over.done = true
    }
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

  for (const tool of ['Artifact', 'Write', 'Read', ...DOCS_ACTS.map(act => `mcp__claude_ai_Claude_Docs__${act}`)]) {
    on('tool.call', { tool }, async ($, e, next) => {
      const ran = await next(e)
      if (ran.deny === undefined && ran.isError !== true) {
        const created = tool === 'Write' && (ran.result as { type?: string } | undefined)?.type === 'create'
        void made($, tool, e as Record<string, unknown>, ran.text ?? '', created).catch(() => undefined)
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
