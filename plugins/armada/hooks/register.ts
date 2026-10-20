// Tells a running Fleet which sessions this machine has open and what each holds.
// `docs/concepts/session.md`.
//
// **A session is reported as it happens and never read back**, with one
// exception: a message a person sent it from Bridge, which Fleet holds until
// this mod asks (`submitHeld`). Two things are told to the model: its own session id, so
// `show_window` can name it, and the lines a stop is sent back with. Four hooks change what a session does: an `open` of a page, a whole read of a large file
// (turned away once with its outline, `reading.ts`), an `AskUserQuestion` Bridge answers first, and a stop on a reply that names what the owner owes outside
// `waiting_for`, sent back once (`sendBack`); every other answers with what `next` returned and lets its report go unawaited, so
// Fleet being down costs the session nothing (`fleet.ts`). **No message text and no prompt leaves, apart from the first
// line of the first prompt as a title, the first line of what a turn ended on as the reply Fleet shows, the first line of a shell command, the text of each item on the task list the agent wrote, a question put to Bridge, and each line a reply named as owed that the session left out of
// `waiting_for` after it was sent back.** A message is reported as who it went to
// or came from, and how many. A file a session wrote or read is reported as its path from the session's directory,
// and never its content. What fills the context window is reported as names, paths and counts, never contents.
// The Loadout is reported the same way: each skill, subagent, MCP server and tool, memory file, output style,
// Armada's rules and this mod's own `armadaSession` block by name and where it came from, and each skill, MCP
// tool, loaded tool and subagent the agent used by name, never a body, an input or a prompt (`loadoutOf`).
//
// The helpers are top-level because the engine follows `$` only into a function
// declared at the top of a file, and refuses the module otherwise.

import type { Engine, Register } from 'claude-code'

import {
  answersIn,
  artifactOf,
  usedFact,
  customTitleIn,
  ghAct,
  ghIssueCreate,
  issuesIn,
  mayMoveBranch,
  micros,
  breakdownOf,
  commitIn,
  madeTask,
  planOf,
  spentUntil,
  pagesOpenedIn,
  pullRequestsIn,
  pushIn,
  senderOf,
  titleOf,
  todoTasks,
  updatedTask,
  storedTasks,
  taskListDir,
  saidOf,
  withoutMarkup,
  effortOf,
  modeOf,
  modelName,
  MOD_VERSION,
  agentItemsIn,
  backgroundEndOf,
  backgroundOf,
  isCovered,
  needsSaidIn,
  sentBackReason,
  waitingItemsOf,
  withReplyNeeds,
  promptDetail,
  transcriptPath,
  agentUseOrigin,
  loadoutOf,
  searchedOf,
  skillUseOrigin,
  styleOrigin,
  toolOrigin,
  waitedOf,
} from './facts'
import type { Equipment, LoadoutItem, LoadoutUse, RateLimit, WaitingItem } from './facts'
import type { Asked, Door, Fact, Report, Task } from './fleet'
import { turnedAway, wholeRead, WHOLE_READ_MOST } from './reading'

const HARNESS = 'claude_code'
const MEASURE_EVERY_MS = 10_000
const RUNTIME_FILE = 'Library/Application Support/Armada/fleet.json'
const WAIT_MS = 1500
// The tick that starts an ask when none is open, so also how soon one is retried after it failed.
const ASK_EVERY_MS = 2000
// How long Fleet may hold an ask, and how long the mod waits on one before it asks again.
const HELD_WAIT_MS = 25_000
const HELD_FETCH_MS = 35_000
const SILENT_MS = 30_000
const DOCS_ACTS = ['create', 'batch', 'update']

type Dollar = Door & Pick<Engine, 'command' | 'process' | 'prompt' | 'session'>

type Known = {
  cwd: string
  title?: string
  branch?: string
  prs: Map<string, Record<string, string>>
  messages: Map<string, number>
  artifacts: Map<string, Record<string, string>>
  /** The tasks the agent started in the background, by task id, with the detail last told (`status` once ended). */
  background: Map<string, Record<string, string>>
  /** The agent's task list as its tools last left it, in order. */
  tasks: Task[]
  /** Whether the harness's own list was read for this process, which is done once. */
  tasksRead: boolean
  /** The list as Fleet last heard it, so a seed that changes nothing Fleet holds tells nothing. */
  tasksTold: string
  tuned?: Tuning
  // The terminal's permission prompts open now, oldest first. Fleet is told of the first.
  prompts: Prompt[]
  /** What the session's last `waiting_for` call set, and whether this mod load saw one. */
  waiting: WaitingItem[]
  sawWaiting: boolean
  turn: Turn
  loadout: Loadout
}

/** What the Loadout is made from, the list Fleet last heard, and when the turn its uses go under began. */
type Loadout = { equipment: Equipment; told?: LoadoutItem[]; turnAt: string }

/**
 * What this turn did about the owner. `sentBack` holds the lines the Stop hook sent the session back
 * with; a turn is sent back once, then what is still missing is put in the list for it.
 */
type Turn = { setWaiting: boolean; asked: boolean; sentBack?: string[]; reported: boolean }

const freshTurn = (): Turn => ({ setWaiting: false, asked: false, reported: false })

type Prompt = { tool: string; detail: string }

type Tuning = Extract<Fact, { kind: 'tuned' }>

type Reading = { tokens?: number; window?: number; usd?: number; limits: RateLimit[]; changed: string[] }

const known = new Map<string, Known>()
let measuredAt = Number.NEGATIVE_INFINITY
/** The newest reading the throttle held back, told when its ten seconds are up. */
let measureHeld: Reading | undefined
/** Files a whole read was turned away from in this load: the second asks for it on purpose. */
const turnedAwayFrom = new Set<string>()

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
    ...[...one.prs].map(([number, detail]) =>
      told(id, { kind: 'attached', attachment: { kind: 'pr', target: number, detail } }),
    ),
    ...[...one.artifacts].map(([target, detail]) =>
      told(id, { kind: 'attached', attachment: { kind: 'artifact', target, detail } }),
    ),
    ...[...one.background].flatMap(([target, detail]) => [
      told(id, { kind: 'attached', attachment: { kind: 'background', target, detail } }),
      ...(detail.status === undefined ? [] : [told(id, { kind: 'settled', attachment: { kind: 'background', target }, state: 'spent' as const })]),
    ]),
    ...(one.tasks.length === 0 ? [] : [told(id, { kind: 'tasks', tasks: one.tasks })]),
    ...one.prompts.slice(0, 1).map(open => told(id, promptFact(open))),
    ...(one.loadout.told === undefined ? [] : [told(id, { kind: 'equipped', loadout: { items: one.loadout.told } })]),
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

// **Slow is not down.** A connection that fails is Fleet out of reach, and a request still unanswered
// after `WAIT_MS` is only Fleet busy: the report is left to land and the next goes behind it.
async function post($: Door, report: Report): Promise<boolean> {
  const port = await portOf($)
  if (port === undefined) return false
  const sent = $.http.fetch(`http://127.0.0.1:${port}/sessions/report`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(report),
  })
  // Handled here too, so a refusal that arrives after the wait is not left unhandled.
  sent.catch(() => undefined)
  await Promise.race([sent, $.clock.sleep(WAIT_MS)])
  return true
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

// What a person sent this session from Bridge. Each is submitted as the person's
// own prompt, which starts a turn when the session is idle and waits for it when
// it is not (spike 27). One at a time, so they keep their order.
let submitting: Promise<void> = Promise.resolve()

// **One ask is held open at a time**, and Fleet answers it the moment something is held, so a
// message arrives when it is sent and not at the next tick. It is its own path: the reports'
// silence does not gate it, and its failures are met here (the tick asks again).
let holding = false

async function submitHeld($: Dollar): Promise<void> {
  if (holding) return
  holding = true
  try {
    while (await askHeld($)) continue
  } finally {
    holding = false
  }
}

/** One held ask. True to ask again at once; false where the tick should try later. */
async function askHeld($: Dollar): Promise<boolean> {
  const port = await portOf($)
  if (port === undefined) return false
  const id = await $.session.id()
  const began = await $.clock.now()
  const asked = await Promise.race([
    $.http.fetch(`http://127.0.0.1:${port}/sessions/held`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ session_id: id, wait_ms: HELD_WAIT_MS }),
    }),
    $.clock.sleep(HELD_FETCH_MS).then(() => undefined),
  ])
  if (asked === undefined || !asked.ok) return false
  const held = JSON.parse(asked.text) as { messages?: unknown; commands?: unknown; end?: unknown }
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
  const messages = Array.isArray(held.messages) ? held.messages : []
  for (const text of messages) {
    if (typeof text !== 'string') continue
    submitting = submitting
      .then(() => $.prompt.submit({ text, asUser: true }))
      .then(() => undefined)
      .catch(() => undefined)
  }
  // End session pressed in Bridge: `/exit`, as the person would type it, behind the messages above.
  // It waits for the session to be idle, so a message submitted just before has its turn first.
  const end = held.end === true
  if (end) {
    submitting = submitting
      .then(() => $.command.run({ command: 'exit' }))
      .then(() => undefined)
      .catch(() => undefined)
  }
  // A Fleet that answers at once with nothing (one from before the wait, or a session it has not
  // heard of yet) is asked no faster than the tick used to.
  const took = (await $.clock.now()) - began
  if (commands.length === 0 && messages.length === 0 && !end && took < ASK_EVERY_MS) {
    await $.clock.sleep(ASK_EVERY_MS - took)
  }
  return true
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

/**
 * The pull request on the branch checked out in `cwd`, or the one `ref` names. `created` marks one this
 * session opened, which Fleet credits wherever the session stood.
 */
async function pullRequestOn($: Dollar, id: string, one: Known, cwd: string, ref?: string, created = false): Promise<void> {
  const ran = await $.process.run(['gh', 'pr', 'view', ...(ref === undefined ? [] : [ref]), '--json', 'number,url,state,title,headRefName,isDraft'], {
    cwd,
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
    ...(created ? { created: 'true' } : {}),
  }
  one.prs.set(number, detail)
  say($, id, { kind: 'attached', attachment: { kind: 'pr', target: number, detail } })
  if (view.state === 'MERGED') settle($, id, 'pr', number, 'spent')
  if (view.state === 'CLOSED') settle($, id, 'pr', number, 'given_back')
}

/**
 * Where the branch is now, and the pull request on it. **Read from where the session is now**, not
 * from the directory last reported: a session that left a slot, or a slot another session leased
 * since, would otherwise be credited with the branch checked out there. Fleet checks it too.
 */
async function look($: Dollar, id: string): Promise<void> {
  try {
    const one = known.get(id)
    if (one === undefined) return
    const cwd = await $.session.cwd()
    const branch = await branchOf($, cwd)
    if (branch === one.branch) return
    if (one.branch !== undefined && branch === undefined) {
      settle($, id, 'branch', one.branch, 'given_back')
    }
    one.branch = branch
    if (branch === undefined) return
    say($, id, { kind: 'attached', attachment: { kind: 'branch', target: branch } })
    await pullRequestOn($, id, one, cwd).catch(() => undefined)
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

/** What is put to Bridge: a question's input, or a call the terminal is asking permission for and its tool. */
type Asking = { input: unknown; tool?: string }

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
 * prompt is all there is meanwhile. `over` ends the loop once the terminal's prompt has. `first` is
 * Fleet's answer to the post the hook made before the prompt opened, where it came in time.
 */
async function putToBridge($: Door, id: string, asking: Asking, over: { done: boolean }, first?: Asked): Promise<Asked | undefined> {
  try {
    if (first !== undefined && first.outcome !== 'asked') return undefined
    let call = first?.outcome === 'asked' ? first.call : undefined
    while (call === undefined && !over.done) {
      const port = await portOf($).catch(() => undefined)
      const put = port === undefined ? undefined : await postAsk($, port, { kind: 'asks', session_id: id, ...asking }, POLL_FETCH_MS).catch(() => undefined)
      if (put?.outcome === 'asked') call = put.call
      else if (put?.outcome === 'refused') return put
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

/**
 * The question's first post, bounded short: it runs before the terminal's prompt opens, so a question
 * Fleet refuses (a walk is open) never opens one. A Fleet slow or out of reach answers nothing here,
 * and `putToBridge` posts again.
 */
async function firstPut($: Door, id: string, asking: Asking): Promise<Asked | undefined> {
  try {
    const port = await portOf($).catch(() => undefined)
    if (port === undefined) return undefined
    return await postAsk($, port, { kind: 'asks', session_id: id, ...asking }, WAIT_MS)
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

/** `ask_in_window`, for a question about the page the walk shows; any other question waits. */
const ASK_IN_WINDOW = "To ask the owner about something on a page you showed him, call ask_in_window with the question, its options, the page's url and a CSS selector for the place, passing session_id, then end your turn.";

/** `show_sketch`, for any diagram, so the owner is never handed ASCII art. */
const SHOW_SKETCH = "To show the owner a diagram, a flow, an architecture or a layout, call show_sketch with a title and a scene, passing session_id; never draw ASCII art.";

function idNote(id: string): string {
  return `Your Armada job id is ${id}. Pass it as session_id to the armada show_window tool. To show the owner a web page (a walk, a mock, a dev server), call show_window; to offer several takes of one change, pass variations, a list of {name, url}. To walk him through a page step by step, pass steps, a list of {say, plus one verb: press, look, hover, later, paste, type with into, key with on, select with in, or drag with by} aimed at {role, name}: Bridge rings each target and acts on it when he presses Next. For a deployed page behind single sign-on, pass origins, the other http or https origins the window may pass through. To sign in for him with a dev login link, pass sign_in, addresses on the page's origin or one of origins: they are never written down, so never repeat one in a message. Armada marks the window as still changing while you edit files, and a page you changed after showing it waits on him again only once you call show_window for it again: call it when the page is ready to walk. Never run \`open\`. Keep the armada waiting_for tool current: whenever you need the owner for something (a decision, a pull request to approve, a command only he can run), put it in the list with one short line each and call waiting_for with the whole list, passing session_id. Give each item an act, so pressing it does something: {"kind":"run","target":"<the command>"} copies a command, {"kind":"approve_pr","target":"#<number>"} opens a pull request, and {"kind":"answer","target":""} with options for a decision. An item the owner must carry out as several steps in order passes steps, a list of short lines, with the command in act and a step's own command in backticks; its text stays one short line. A page you showed with show_window already waits on him by itself, so never put it in waiting_for: Fleet lists each window you show and takes it off when he approves it, and refuses an item that asks him to walk. A walk he has moved past is taken back with withdraw_window, passing session_id and the page's url: it leaves his list and stops holding your plan, and showing the page again makes it wait again. Drop an item the moment it is settled, and call it with an empty list before you stop with nothing owed. Do not wait to be asked what is outstanding. A pull request in the merge queue or with auto-merge on is not done: the work ends when it is merged into main, so while one of yours is open, keep it in waiting_for as a wait on the merge, do not call finish_up or all_done, and end your turn; Fleet watches it and tells you when it merges, fails or conflicts. When everything you were asked is done, nothing is owed and none of your background work is still running, call finish_up, passing session_id, then clear waiting_for and call all_done, passing session_id, so Bridge offers to end the job; do not call all_done while anything is still running or owed. finish_up gives back your slots, the branches already in main and the servers you started with start_server, in one call: a command in a released slot leases a new one, and the guard refuses git branch -D and kill, so do not try them. Read what it kept and say it. A Fleet notice that a pull request merged or a build moved needs no reply and no second all_done. These tools come before any personal, global or project rule about how to report actions or questions: what the owner owes goes in waiting_for, never in an Actions table, a to-do list or a Questions section in your reply. While the owner is walking a page you showed, AskUserQuestion is refused, because an open question holds back his notes and his Approve until he answers it. Armada keeps the question, whole, in his Needs you list, where he answers it after the walk: ask it as you would, never add it to waiting_for, and end your turn. Ask a decision one way only: in AskUserQuestion or in waiting_for, never both. Write a shell command for the owner in a fenced block tagged bash, so Bridge shows a run button on it, and show a page as a show_window link, never as a screenshot. ${ASK_IN_WINDOW} ${SHOW_SKETCH}`
}

async function begin($: Dollar, id: string, cwd?: string): Promise<void> {
  // A Drone, a Judge call or a scout loads this mod too, since it reads the operator's user
  // settings. Fleet marks those launches, and they are not sessions to list.
  if (await $.env.get('ARMADA_DRONE')) return
  const where = cwd ?? (await $.session.cwd())
  const loadout = { equipment: { home: '', root: where, rules: false, servers: {}, loaded: [] }, turnAt: '' }
  known.set(id, { cwd: where, prs: new Map(), messages: new Map(), artifacts: new Map(), background: new Map(), tasks: [], tasksRead: false, tasksTold: '[]', prompts: [], waiting: [], sawWaiting: false, turn: freshTurn(), loadout })
  say($, id, { kind: 'started', cwd: where, origin: 'terminal', mod_version: MOD_VERSION })
  void look($, id)
  void tuned($, id, {}, true).catch(() => undefined)
  void equipped($, id).catch(() => undefined)
}

/** The Loadout's first reading: where the session's files are, whether Fleet hosts it, and the window's listings. */
async function equipped($: Dollar, id: string): Promise<void> {
  const one = known.get(id)
  if (one === undefined) return
  const equipment = one.loadout.equipment
  if (one.loadout.turnAt === '') one.loadout.turnAt = new Date(await $.clock.now()).toISOString()
  equipment.home = (await $.env.get('HOME')) ?? ''
  equipment.root = await $.session.root().catch(() => equipment.root)
  equipment.rules = Boolean(await $.env.get('ARMADA_SESSION'))
  await equip($, id, true)
}

/** The Loadout, told when it moved; `read` takes the window's listings afresh first. */
async function equip($: Dollar, id: string, read = false): Promise<void> {
  const one = known.get(id)
  if (one === undefined) return
  if (read) one.loadout.equipment.breakdown = (await breakdownNow($)) ?? one.loadout.equipment.breakdown
  const items = loadoutOf(one.loadout.equipment)
  if (JSON.stringify(items) === JSON.stringify(one.loadout.told)) return
  one.loadout.told = items
  say($, id, { kind: 'equipped', loadout: { items } })
}

/** One skill, MCP tool, loaded tool or subagent the agent used, under the turn it was used in. */
async function equipmentUsed($: Dollar, use: Omit<LoadoutUse, 'turn_started_at'> | undefined): Promise<void> {
  if (use === undefined) return
  const [id, one] = await current($)
  say($, id, { kind: 'used', use: { turn_started_at: one.loadout.turnAt, ...use } })
}

/** What a hook learned of the servers and the loaded tools, folded in and told if it moved anything. */
async function reequipped($: Dollar, servers: Equipment['servers'], loaded: string[] = []): Promise<void> {
  const [id, one] = await current($)
  const equipment = one.loadout.equipment
  equipment.servers = { ...equipment.servers, ...servers }
  equipment.loaded = [...new Set([...equipment.loaded, ...loaded])]
  await equip($, id)
}

/**
 * The output style a system prompt was composed with. A teammate's or an analysis prompt is not the
 * session's own; the default style is no item.
 */
async function styled($: Dollar, name: string | undefined): Promise<void> {
  const one = known.get(await $.session.id())
  if (one === undefined || one.loadout.equipment.style?.name === name) return
  const equipment = one.loadout.equipment
  if (name === undefined) {
    equipment.style = undefined
  } else {
    const inRepository = await $.fs.exists(`${equipment.root}/.claude/output-styles/${name}.md`).catch(() => false)
    equipment.style = { name, ...styleOrigin(name, inRepository) }
  }
  await equip($, await $.session.id())
}

/** A use with its source, or nothing where Armada has no source to give it. */
function sourced(use: Omit<LoadoutUse, 'turn_started_at' | 'source' | 'from'>, origin: Pick<LoadoutUse, 'source' | 'from'> | undefined) {
  return origin === undefined ? undefined : { ...use, ...origin }
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
  const act = ghAct(command)
  if (act?.act === 'create') {
    // What `gh pr create` printed is this session's own, whichever slot the command ran in.
    for (const pr of pullRequestsIn(text)) {
      const detail = { url: pr.url, created: 'true' }
      one.prs.set(pr.number, detail)
      say($, id, { kind: 'attached', attachment: { kind: 'pr', target: pr.number, detail } })
      await pullRequestOn($, id, one, await $.session.cwd(), pr.url, true).catch(() => undefined)
    }
  }
  // What `gh issue create` printed is an issue the session filed. Fleet keeps it only where it is in
  // the session's own repository, and reads its title and state from the forge.
  if (ghIssueCreate(command)) {
    for (const issue of issuesIn(text)) {
      say($, id, { kind: 'attached', attachment: { kind: 'issue', target: issue.number, detail: { address: issue.url } } })
    }
  }
  if (act?.act === 'merge' || act?.act === 'close') {
    const target = act.number ?? (one.prs.size === 1 ? [...one.prs.keys()][0] : undefined)
    if (target !== undefined) {
      settle($, id, 'pr', target, act.act === 'merge' ? 'spent' : 'given_back')
    }
  }
  // Read from what the commands printed, so a commit with nothing staged and a push with nothing to
  // send are not told. Fleet enters them on the timeline, where the build record reads its road.
  const committed = commitIn(command, text)
  if (committed !== undefined) say($, id, { kind: 'committed', ...committed })
  const pushed = pushIn(command, text)
  if (pushed !== undefined) say($, id, { kind: 'pushed', ...pushed })
  if (mayMoveBranch(command) || act?.act === 'create') {
    const cwd = await $.session.cwd()
    if (cwd !== one.cwd) {
      one.cwd = cwd
      say($, id, { kind: 'moved', cwd })
    }
    await look($, id)
  }
}

/**
 * The agent's task list, whole, after one of its task tools ran. A subagent's `TaskCreate` and `TaskUpdate`
 * write the same list (measured on claude 2.1.300), so they are heard too, each task marked with who made it.
 * A subagent's `TodoWrite` is a list of its own and would replace this one: not heard. A list that did not
 * change is not told again.
 *
 * **A process that began after the list was made does not hold it** (a session resumed after Fleet restarted,
 * or moved into a slot), so the first of these tools reads the harness's own list from disk (`stored`) and
 * the call's change goes on top of it. Without that, a `TaskUpdate` named a task nothing here had made and
 * was never told. The `TaskCreate` hears `by` from the call; a task made before this process has none.
 */
async function planned($: Dollar, next: (list: Task[]) => Task[] | undefined, seed = true): Promise<void> {
  const [id, one] = await current($)
  if (seed && !one.tasksRead) {
    const kept = await stored($, id).catch(() => undefined)
    if (kept !== undefined && !one.tasksRead) {
      one.tasksRead = true
      if (one.tasks.length === 0) one.tasks = kept
    }
  }
  const tasks = next(one.tasks) ?? one.tasks
  one.tasks = tasks
  const now = JSON.stringify(tasks)
  if (now === one.tasksTold) return
  one.tasksTold = now
  say($, id, { kind: 'tasks', tasks })
}

/** The harness's own list for this session, or nothing where it has none yet or cannot be read. */
async function stored($: Dollar, id: string): Promise<Task[] | undefined> {
  const home = await $.env.get('HOME')
  if (!home) return undefined
  const dir = taskListDir(home, (await $.env.get('CLAUDE_CODE_TASK_LIST_ID')) || id)
  const entries = await $.fs.list(dir)
  const files = await Promise.all(
    entries
      .filter(one => one.kind === 'file' && one.name.endsWith('.json') && !one.name.startsWith('.'))
      .map(one => $.fs.read(`${dir}/${one.name}`).then(text => (typeof text === 'string' ? text : ''), () => '')),
  )
  return storedTasks(files)
}

async function spawned(
  $: Dollar,
  target: string,
  type: string,
  description: string,
  call?: string,
): Promise<void> {
  const [id] = await current($)
  // The call that started it: Fleet keys a hosted subagent's touches by it, the Field matches them to this row by it.
  const detail = { type, description: description.slice(0, 80), ...(call === undefined || call === target ? {} : { call }) }
  say($, id, { kind: 'attached', attachment: { kind: 'subagent', target, detail } })
}

/**
 * A shell command run in the background, or a Monitor, as the call's own result names it. **Told on the
 * call and settled on the engine's notice**, and never on what the agent says of it.
 */
async function backgrounded(
  $: Dollar,
  tool: string,
  input: Record<string, unknown>,
  text: string,
): Promise<void> {
  const task = backgroundOf(tool, input, text)
  if (task === undefined) return
  const [id, one] = await current($)
  one.background.set(task.target, task.detail)
  say($, id, { kind: 'attached', attachment: { kind: 'background', target: task.target, detail: task.detail } })
}

/**
 * A background task ended: by the engine's notice (`completed`, `failed`, or `killed` for one stopped), or
 * by a `TaskStop` the agent called. Only a task this load saw start is settled, and once.
 */
async function backgroundEnded($: Dollar, target: string, word: string, exit?: string, output?: string): Promise<void> {
  const [id, one] = await current($)
  const held = one.background.get(target)
  if (held === undefined || held.status !== undefined) return
  const status = word === 'completed' ? 'completed' : word === 'killed' || word === 'stopped' ? 'killed' : 'failed'
  const detail = { ...held, status, ...(exit === undefined || status === 'killed' ? {} : { exit }), ...(output === undefined ? {} : { output }) }
  one.background.set(target, detail)
  say($, id, { kind: 'attached', attachment: { kind: 'background', target, detail } })
  settle($, id, 'background', target, 'spent')
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

async function used(
  $: Dollar,
  act: 'touched' | 'read' | 'ran',
  input: Record<string, unknown>,
  by?: { how?: 'write' | 'edit'; agent?: string },
): Promise<void> {
  const [id, one] = await current($)
  const fact = usedFact(act, one.cwd, input, by)
  if (fact !== undefined) say($, id, fact)
}

async function messaged($: Dollar, direction: 'sent' | 'received', who: string): Promise<void> {
  const [id, one] = await current($)
  const target = `${direction === 'sent' ? 'to' : 'from'}:${who}`
  const count = (one.messages.get(target) ?? 0) + 1
  one.messages.set(target, count)
  const detail = { direction, count: String(count) }
  say($, id, { kind: 'attached', attachment: { kind: 'message', target, detail } })
}

async function measured(
  $: Dollar,
  tokens?: number,
  window?: number,
  usd?: number,
  limits: RateLimit[] = [],
  changed: string[] = [],
): Promise<void> {
  const at = await $.clock.now()
  const held = at - measuredAt < MEASURE_EVERY_MS
  // A window that moved is the one reading nothing else reports, so it is not left behind by the throttle.
  if (held && !changed.includes('rateLimits')) {
    hold($, measuredAt + MEASURE_EVERY_MS - at, { tokens, window, usd, limits, changed })
    return
  }
  measuredAt = at
  measureHeld = undefined
  const [id, one] = await current($)
  const usage = { context_tokens: tokens, context_window: window, cost_micros: micros(usd) }
  const plan = planOf(limits)
  const fact: Extract<Fact, { kind: 'measured' }> = { kind: 'measured', usage }
  if (plan.length > 0) fact.plan = plan
  // `summary` estimates locally; `full` would call the token-count API for every tool and file.
  const found = !held && changed.includes('context') ? await breakdownNow($) : undefined
  if (found !== undefined) fact.context_breakdown = breakdownOf(found)
  say($, id, fact)
  if (found !== undefined) {
    one.loadout.equipment.breakdown = found
    await equip($, id)
  }
}

/**
 * Keeps a reading the throttle held back, over any older one, and tells it when the window is up,
 * so the last reading of a burst is not left behind until something else is measured.
 */
function hold($: Dollar, wait: number, reading: Reading): void {
  const older = measureHeld
  measureHeld = {
    tokens: reading.tokens ?? older?.tokens,
    window: reading.window ?? older?.window,
    usd: reading.usd ?? older?.usd,
    limits: reading.limits.length > 0 ? reading.limits : (older?.limits ?? []),
    changed: [...new Set([...(older?.changed ?? []), ...reading.changed])],
  }
  if (older !== undefined) return
  void $.clock
    .sleep(wait)
    .then(() => {
      const one = measureHeld
      if (one === undefined) return
      measureHeld = undefined
      return measured($, one.tokens, one.window, one.usd, one.limits, one.changed)
    })
    .catch(() => undefined)
}

/** What one main-thread model request was answered over: uncached, cache-written and cache-read. */
function stepTokens(usage: { input_tokens: number; cache_creation_input_tokens: number; cache_read_input_tokens: number }): number {
  return usage.input_tokens + usage.cache_creation_input_tokens + usage.cache_read_input_tokens
}

async function breakdownNow($: Dollar) {
  try {
    return (await $.session.usage({ breakdown: 'summary' })).context.breakdown
  } catch {
    return undefined
  }
}

/**
 * A turn ended on the plan's limit. Fleet is told when the window that is full opens again, and
 * wakes the session with `Continue` then; no window at or near full leaves nothing to wake it at.
 */
async function limited($: Dollar): Promise<void> {
  const [id] = await current($)
  const resets = spentUntil((await $.session.usage()).rateLimits)
  if (resets !== undefined) say($, id, { kind: 'limited', resets_at: resets })
}

async function titled($: Dollar, prompt: string): Promise<void> {
  const [id, one] = await current($)
  const title = one.title === undefined ? titleOf(prompt) : undefined
  if (title === undefined) return
  one.title = title
  say($, id, { kind: 'titled', title })
}

/**
 * A turn began, so Fleet has the session working and the reply the turn before left is over. **Fleet
 * reads this from the prompt going in and never from the model.** A slash command is not a turn:
 * most of them end without one, so they would leave the session working with nothing to end it.
 */
async function turnBegan($: Dollar, prompt: string): Promise<void> {
  if (withoutMarkup(prompt).trim().startsWith('/')) return
  const [id, one] = await current($)
  one.prompts = []
  one.loadout.turnAt = new Date(await $.clock.now()).toISOString()
  say($, id, { kind: 'turn_started' })
}

/** A turn ended: Fleet has the first line of what it ended on as the reply, with no call from the model. */
function turnEnd(answer: string): Fact {
  const said = saidOf(answer)
  return said === undefined ? { kind: 'turn_completed' } : { kind: 'turn_completed', said }
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

function promptFact(open: Prompt): Fact {
  return open.detail === '' ? { kind: 'permission_asked', tool: open.tool } : { kind: 'permission_asked', tool: open.tool, detail: open.detail }
}

/**
 * The harness opened its own permission prompt, so the session waits on a person at the terminal.
 * **The harness says so**, through its `PermissionRequest` hook; the agent is never asked. Fleet is
 * told of the oldest open, which is the one the terminal shows.
 */
async function promptOpened($: Dollar, tool: string, input: unknown): Promise<void> {
  const [id, one] = await current($)
  const open = { tool, detail: promptDetail((input ?? {}) as Record<string, unknown>) }
  one.prompts.push(open)
  if (one.prompts.length === 1) say($, id, promptFact(open))
}

/**
 * A tool call ended, so the prompt it raised is closed: it was answered and ran, or was refused. The
 * next one open, if any, is told in its place.
 */
async function promptSettled($: Dollar, tool: string, input: Record<string, unknown>): Promise<void> {
  const id = await $.session.id()
  const one = known.get(id)
  if (one === undefined || one.prompts.length === 0) return
  const detail = promptDetail(input)
  const at = one.prompts.findIndex(open => open.tool === tool && open.detail === detail)
  if (at < 0) return
  one.prompts.splice(at, 1)
  if (at !== 0) return
  say($, id, { kind: 'permission_settled' })
  const next = one.prompts[0]
  if (next !== undefined) say($, id, promptFact(next))
}

/** A tool call under way whose terminal prompt Bridge may answer first. */
type Underway = { answer: (asked: Asked) => void; over: { done: boolean }; put: boolean }

// The calls under way by `callKey`, oldest first, and the ones Bridge allowed, which `tool.check` lets
// run again without the terminal's prompt.
const underway = new Map<string, Underway[]>()
const allowedInBridge = new Set<string>()

function callKey(tool: string, input: Record<string, unknown>): string {
  return `${tool}\n${promptDetail(input)}`
}

/**
 * The terminal's prompt, put to Bridge as well: the call that raised it takes Bridge's answer where it
 * comes first. Fleet keeps it until a person answers or the call ends.
 */
async function promptPut($: Dollar, tool: string, input: unknown): Promise<void> {
  const id = await $.session.id()
  if (!known.has(id)) return
  const call = underway.get(callKey(tool, (input ?? {}) as Record<string, unknown>))?.find(one => !one.put)
  if (call === undefined) return
  call.put = true
  const asked = await putToBridge($, id, { tool, input }, call.over)
  if (asked?.outcome === 'answered' || asked?.outcome === 'refused') call.answer(asked)
}

/**
 * A turn is stopping on a reply that names what the owner owes and a `waiting_for` list that does not
 * hold it. The first stop is blocked with the lines found; the second, or one another hook already sent
 * back (`stopActive`), is let through and the lines go in the list marked as read from the reply, so
 * the session can never loop on it.
 */
async function sendBack($: Dollar, answer: string, stopActive: boolean): Promise<string | undefined> {
  if (await $.env.get('ARMADA_DRONE')) return undefined
  const [id, one] = await current($)
  const turn = one.turn
  if (turn.reported) return undefined
  const found = needsSaidIn(answer)
  if (turn.sentBack === undefined && !stopActive) {
    if (found.length === 0 || turn.asked || isCovered(found, one.waiting)) return undefined
    turn.sentBack = found
    turn.setWaiting = false
    return sentBackReason(found, id)
  }
  turn.reported = true
  const needs = [...new Set([...(turn.sentBack ?? []), ...found])]
  const complied = turn.sentBack !== undefined && turn.setWaiting && one.waiting.length > 0
  if (needs.length === 0 || turn.asked || complied || isCovered(needs, one.waiting)) return undefined
  void putInWaiting($, id, one, needs).catch(() => undefined)
  return undefined
}

/**
 * The session's list with the reply's lines added, set through the route the `waiting_for` tool posts to.
 * **The post replaces the list, so it starts from the one Fleet holds**; where that cannot be read, from
 * the mod's own copy only if this load saw the agent set it, and otherwise nothing is posted.
 */
async function putInWaiting($: Door, id: string, one: Known, needs: string[]): Promise<void> {
  const port = await portOf($)
  if (port === undefined) return
  const read = await Promise.race([
    $.http.fetch(`http://127.0.0.1:${port}/sessions/one?session_id=${encodeURIComponent(id)}`),
    $.clock.sleep(WAIT_MS).then(() => undefined),
  ]).catch(() => undefined)
  const held = read?.ok === true ? agentItemsIn(read.text) : undefined
  const base = held ?? (one.sawWaiting ? one.waiting : undefined)
  if (base === undefined) return
  const items = withReplyNeeds(base, needs)
  if (items.length === base.length) return
  const sent = await Promise.race([
    $.http.fetch(`http://127.0.0.1:${port}/sessions/waiting`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ session_id: id, items }),
    }),
    $.clock.sleep(WAIT_MS).then(() => undefined),
  ])
  if (sent?.ok === true) one.waiting = items
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
    const one = known.get(await $.session.id())
    if (one !== undefined) one.turn = freshTurn()
    void titled($, e.text).catch(() => undefined)
    void turnBegan($, e.text).catch(() => undefined)
    // Whatever the origin: the notice can arrive inside a system-reminder, and only a task this load saw start is settled.
    const ended = backgroundEndOf(e.text)
    if (ended !== undefined) void backgroundEnded($, ended.target, ended.status, ended.exit, ended.output).catch(() => undefined)
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
    if (ran.deny === undefined) void used($, 'ran', e as Record<string, unknown>).catch(() => undefined)
    if (ran.deny === undefined && ran.isError !== true) {
      void afterBash($, e.command, ran.text ?? '').catch(() => undefined)
      void backgrounded($, 'Bash', e as Record<string, unknown>, ran.text ?? '').catch(() => undefined)
    }
    return ran
  })

  // A whole read of a large file is turned away once, with its outline, so the agent reads a range.
  on('tool.call', { tool: 'Read' }, async ($, e, next) => {
    if (!wholeRead(e) || turnedAwayFrom.has(e.file_path)) return next(e)
    const file = await $.fs.stat(e.file_path).catch(() => undefined)
    if (file?.kind !== 'file' || file.size <= WHOLE_READ_MOST) return next(e)
    const text = await $.fs.read(e.file_path).catch(() => undefined)
    if (typeof text !== 'string') return next(e)
    turnedAwayFrom.add(e.file_path)
    const root = await $.session.root().catch(() => undefined)
    const graph = root !== undefined && (await $.fs.exists(`${root}/.gitnexus`).catch(() => false))
    return { deny: turnedAway(e.file_path, file.size, text, graph) }
  })

  // A Monitor watch is a background task as a command run in the background is.
  on('tool.call', { tool: 'Monitor' }, async ($, e, next) => {
    const ran = await next(e)
    if (ran.deny === undefined && ran.isError !== true) {
      void backgrounded($, 'Monitor', e as Record<string, unknown>, ran.text ?? '').catch(() => undefined)
    }
    return ran
  })

  // Stopped by the agent: the notice that follows says so too, and settles nothing twice.
  on('tool.call', { tool: 'TaskStop' }, async ($, e, next) => {
    const ran = await next(e)
    const target = (e as { task_id?: string; shell_id?: string }).task_id ?? (e as { shell_id?: string }).shell_id
    if (ran.deny === undefined && ran.isError !== true && target !== undefined) {
      void backgroundEnded($, target, 'killed').catch(() => undefined)
    }
    return ran
  })

  // A question asked in the terminal is also put to Bridge, and the first answer wins; Fleet refusing it
  // (the owner is walking a page) stops it before the terminal's prompt opens. The terminal's
  // prompt opens beneath (`next`) while Fleet holds the question; returning before `next` does aborts
  // the prompt, so an answer from Bridge is the tool's own result and the terminal never keeps it.
  on('tool.call', { tool: 'AskUserQuestion' }, async ($, e, next) => {
    if (await $.env.get('ARMADA_DRONE')) return next(e)
    const id = await $.session.id()
    const put = await firstPut($, id, { input: { questions: e.questions } })
    if (put?.outcome === 'refused') return { deny: put.message }
    const asking = known.get(id)
    if (asking !== undefined) asking.turn.asked = true
    const over = { done: false }
    const terminal = next(e).then(ran => ({ ran }))
    const bridge = putToBridge($, id, { input: { questions: e.questions } }, over, put).then(asked =>
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

  // The plan, from the tools the agent already calls to write it: nothing is asked of the model. The task
  // tools came in two generations, a whole list in `TodoWrite` and one task at a time in `TaskCreate` and `TaskUpdate`.
  // Only the session's own `TodoWrite` is heard, but a subagent's task calls share the session's list.
  on('tool.call', { tool: 'TodoWrite' }, async ($, e, next) => {
    const ran = await next(e)
    if (e.agentId === undefined && ran.deny === undefined && ran.isError !== true) {
      const list = todoTasks(e as Record<string, unknown>)
      if (list !== undefined) void planned($, () => list, false).catch(() => undefined)
    }
    return ran
  })

  on('tool.call', { tool: 'TaskCreate' }, async ($, e, next) => {
    const ran = await next(e)
    if (ran.deny === undefined && ran.isError !== true) {
      void planned($, list => madeTask(list, e as Record<string, unknown>, ran.result, ran.text ?? '', e.agentId)).catch(() => undefined)
    }
    return ran
  })

  on('tool.call', { tool: 'TaskUpdate' }, async ($, e, next) => {
    const ran = await next(e)
    if (ran.deny === undefined && ran.isError !== true) {
      void planned($, list => updatedTask(list, e as Record<string, unknown>)).catch(() => undefined)
    }
    return ran
  })

  // Plan mode left, on the call that leaves it. A plan the person turned down is an error or a refusal, and is not told.
  on('tool.call', { tool: 'ExitPlanMode' }, async ($, e, next) => {
    const ran = await next(e)
    if (e.agentId === undefined && ran.deny === undefined && ran.isError !== true) {
      void current($)
        .then(([id]) => say($, id, { kind: 'plan_left' }))
        .catch(() => undefined)
    }
    return ran
  })

  // The terminal's own permission prompt is told to Fleet, which raises it as a call and does not read
  // the session as stuck. Told only where a prompt opens: a hook beneath that decided opens none.
  // **Only told**: the terminal's prompt is what answers it.
  on('classic.PermissionRequest', async ($, e, next) => {
    const out = await next(e)
    if (out.decision === undefined) {
      void promptOpened($, e.tool_name, e.tool_input).catch(() => undefined)
      void promptPut($, e.tool_name, e.tool_input).catch(() => undefined)
    }
    return out
  })

  // The terminal's prompt opens beneath (`next`) while Bridge holds it too, and the first answer wins.
  // Allowed in Bridge, the call runs again with `tool.check` allowing it, and returning drops the
  // terminal's prompt (measured on Claude Code 2.1.300, 14 Oct 2026); refused there, it is denied.
  // The call ending closes the prompt it raised, whichever way it was answered.
  on('tool.call', async ($, e, next) => {
    const input = e as Record<string, unknown>
    const key = callKey(e.tool, input)
    let answer: (asked: Asked) => void = () => undefined
    const bridge = new Promise<Asked>(resolve => (answer = resolve))
    const call: Underway = { answer, over: { done: false }, put: false }
    underway.set(key, [...(underway.get(key) ?? []), call])
    try {
      const terminal = next(e).then(ran => ({ ran }))
      // Dropped when Bridge answers first, so its end is not left unhandled.
      terminal.catch(() => undefined)
      const first = await Promise.race([terminal, bridge.then(asked => ({ asked }))])
      if ('ran' in first) {
        if (call.put) void settledInTerminal($, await $.session.id(), first.ran.deny === undefined && first.ran.isError !== true)
        return first.ran
      }
      if (first.asked.outcome === 'refused') return { deny: first.asked.message }
      allowedInBridge.add(key)
      try {
        return await next(e)
      } finally {
        allowedInBridge.delete(key)
      }
    } finally {
      call.over.done = true
      const left = (underway.get(key) ?? []).filter(one => one !== call)
      if (left.length === 0) underway.delete(key)
      else underway.set(key, left)
      void promptSettled($, e.tool, input).catch(() => undefined)
    }
  })

  on('tool.check', async ($, e, next) =>
    allowedInBridge.has(callKey(e.tool, (e.input ?? {}) as Record<string, unknown>)) ? { decision: 'allow' as const } : next(e),
  )

  for (const tool of ['Artifact', 'Write', 'Read', ...DOCS_ACTS.map(act => `mcp__claude_ai_Claude_Docs__${act}`)]) {
    on('tool.call', { tool }, async ($, e, next) => {
      const ran = await next(e)
      if (ran.deny === undefined && ran.isError !== true) {
        const created = tool === 'Write' && (ran.result as { type?: string } | undefined)?.type === 'create'
        void made($, tool, e as Record<string, unknown>, ran.text ?? '', created).catch(() => undefined)
        if (tool === 'Read') void used($, 'read', e as Record<string, unknown>).catch(() => undefined)
      }
      return ran
    })
  }

  // Which files this session writes, for Fleet's check of who else writes them. A path, never content.
  for (const tool of ['Write', 'Edit', 'MultiEdit', 'NotebookEdit']) {
    on('tool.call', { tool }, async ($, e, next) => {
      const ran = await next(e)
      if (ran.deny === undefined && ran.isError !== true) {
        const by = { how: tool === 'Write' ? ('write' as const) : ('edit' as const), agent: e.agentId }
        void used($, 'touched', e as Record<string, unknown>, by).catch(() => undefined)
      }
      return ran
    })
  }

  // The session's own write, not a subagent's, says it is changing what a window shows: Fleet marks
  // the windows it showed as still changing until its turn ends or it shows the page again.
  for (const tool of ['Edit', 'Write', 'NotebookEdit']) {
    on('tool.call', { tool }, async ($, e, next) => {
      const ran = await next(e)
      if (e.agentId === undefined && ran.deny === undefined && ran.isError !== true) {
        void current($)
          .then(([id]) => say($, id, { kind: 'wrote' }))
          .catch(() => undefined)
      }
      return ran
    })
  }

  on('agent.spawn', async ($, e, next) => {
    const out = await next(e)
    if (out.deny === undefined) {
      const target = out.agentId ?? e.tool_use_id
      void spawned($, target, e.subagentType, e.description, e.tool_use_id).catch(() => undefined)
      void current($)
        .then(([, one]) => {
          const use = { kind: 'subagent' as const, name: e.subagentType, ...(e.model === undefined ? {} : { model: e.model }), ...(e.parentAgentId === undefined ? {} : { by: e.parentAgentId }) }
          return equipmentUsed($, sourced(use, agentUseOrigin(e.subagentType, e.provider?.plugin ?? 'engine', one.loadout.equipment.breakdown)))
        })
        .catch(() => undefined)
    }
    return out
  })

  // The Loadout's uses. An ordinary built-in tool is on the timeline already, so only a skill, an MCP
  // tool and what a ToolSearch loaded are told here, and a subagent at its spawn.
  on('tool.call', { tool: 'Skill' }, async ($, e, next) => {
    const ran = await next(e)
    if (ran.deny === undefined) {
      const by = e.agentId === undefined ? {} : { by: e.agentId }
      void current($)
        .then(([, one]) => equipmentUsed($, sourced({ kind: 'skill', name: e.skill, ...by }, skillUseOrigin(e.skill, one.loadout.equipment.breakdown))))
        .catch(() => undefined)
    }
    return ran
  })

  on('tool.call', { tool: /^mcp__/ }, async ($, e, next) => {
    const ran = await next(e)
    if (ran.deny === undefined) {
      const by = e.agentId === undefined ? {} : { by: e.agentId }
      void current($)
        .then(([, one]) => equipmentUsed($, { kind: 'tool', name: e.tool, ...toolOrigin(e.tool, one.loadout.equipment.breakdown?.mcpTools), ...by }))
        .catch(() => undefined)
    }
    return ran
  })

  on('tool.call', { tool: 'ToolSearch' }, async ($, e, next) => {
    const ran = await next(e)
    if (ran.deny === undefined && ran.isError !== true) {
      const { matches, servers } = searchedOf(ran.result)
      const by = e.agentId === undefined ? {} : { by: e.agentId }
      void current($)
        .then(async ([, one]) => {
          for (const name of matches) {
            await equipmentUsed($, { kind: 'tool', name, ...toolOrigin(name, one.loadout.equipment.breakdown?.mcpTools), ...by })
          }
          await reequipped($, servers, matches)
        })
        .catch(() => undefined)
    }
    return ran
  })

  on('tool.call', { tool: 'WaitForMcpServers' }, async ($, e, next) => {
    const ran = await next(e)
    if (ran.deny === undefined && ran.isError !== true) void reequipped($, waitedOf(ran.result)).catch(() => undefined)
    return ran
  })

  on('prompt.compose', async ($, e, next) => {
    const out = await next(e)
    if (!e.traits.includes('teammate') && !e.traits.includes('analysis')) void styled($, e.outputStyle?.name).catch(() => undefined)
    return out
  })

  // Its own transcript can end on a line that says it is still working (13 Oct 2026), so the
  // engine's word that it stopped is what settles it.
  on('classic.SubagentStop', async ($, e, next) => {
    const out = await next(e)
    void current($)
      .then(([id]) => settle($, id, 'subagent', e.agent_id, 'spent'))
      .catch(() => undefined)
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
    void measured($, e.context.tokens, e.context.window, e.cost?.usd, e.rateLimits, e.changed).catch(
      () => undefined,
    )
    return out
  })

  // The engine measures at a turn's end and when a plan window moves, so a long turn would go unread
  // between them: each main-thread request is a reading of the context.
  on('turn.step', async function* ($, e, next) {
    const out = yield* next(e)
    if (e.agentId === undefined && out.usage !== null) {
      void measured($, stepTokens(out.usage)).catch(() => undefined)
    }
    return out
  })

  // The terminal's permission mode and effort are on the settings-hook inputs and nowhere else.
  on('classic.UserPromptSubmit', async ($, e, next) => {
    const out = await next(e)
    void seen($, e.permission_mode, e.effort)
    return out
  })

  // What the owner owes is read from the reply here, where the stop can still be turned back.
  on('classic.Stop', async ($, e, next) => {
    const out = await next(e)
    void seen($, e.permission_mode, e.effort)
    if (e.agent_id !== undefined || out.block !== undefined) return out
    const reason = await sendBack($, e.last_assistant_message ?? '', e.stop_hook_active).catch(() => undefined)
    return reason === undefined ? out : { ...out, block: reason }
  })

  // The list a session keeps in Bridge is what the Stop hook holds a reply to.
  on('tool.call', { tool: 'mcp__armada-fleet__waiting_for' }, async ($, e, next) => {
    const ran = await next(e)
    if (ran.deny === undefined && ran.isError !== true) {
      const items = waitingItemsOf(e as Record<string, unknown>)
      const one = known.get(await $.session.id())
      if (items !== undefined && one !== undefined) {
        one.waiting = items
        one.sawWaiting = true
        one.turn.setWaiting = true
      }
    }
    return ran
  })


  // A turn that died on an API error finished nothing, and Fleet reads a finished turn as the end
  // of a hold for the limit, so the failed one is not told.
  on('classic.StopFailure', async ($, e, next) => {
    const out = await next(e)
    if (e.error === 'rate_limit' && !(await $.env.get('ARMADA_DRONE'))) void limited($).catch(() => undefined)
    return out
  })

  on('turn.complete', async ($, e, next) => {
    const out = await next(e)
    if (e.agentId === undefined && e.reason !== 'error') {
      void current($)
        .then(([id, one]) => {
          one.prompts = []
          say($, id, turnEnd(e.answer))
        })
        .catch(() => undefined)
    }
    return out
  })

  on('session.end', async ($, e, next) => {
    const out = await next(e)
    if (await $.env.get('ARMADA_DRONE')) return out
    await ended($, e.sessionId, e.reason).catch(() => undefined)
    return out
  })
}
