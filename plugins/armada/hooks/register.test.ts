import { expect, mock, test } from 'claude-code/testing'
import type { On } from 'claude-code'

import { MOD_VERSION } from './facts'

// Fleet as the mod meets it: a runtime file naming a port, and whatever answers
// there. The engine's own `$` calls are what the test answers.
const RUNNING = '{"protocol_id":"0000000000000000","pid":1,"port":4242,"started_at":"x"}'
const URL = 'https://github.com/Mele-Labs/armada/pull/1853'

type Posted = { url: string; body: any }

function world(
  on: On,
  options: { running?: () => boolean; slowReports?: boolean; heldGate?: Promise<void>; held?: string[]; heldCommands?: { command: string; args: string }[]; model?: () => string; onCommand?: () => void; transcript?: string; env?: Record<string, string>; pullRequest?: string; polls?: (string | Promise<string> | Error)[] } = {},
) {
  const running = options.running ?? (() => true)
  const posts: Posted[] = []
  const attempts = { fetches: 0, reads: 0, reports: 0 }
  const submitted: { text: string; asUser?: boolean }[] = []
  const asked: unknown[] = []
  const questions: any[] = []
  const ran: { command: string; args?: string }[] = []
  const clock = mock.clock(on, { now: 1_000_000 })
  mock.env(on, { HOME: '/home/user', ...options.env })
  on('fs.read', (_$, e) => {
    attempts.reads += 1
    if (!running()) throw new Error('no such file')
    if (e.path.endsWith('.jsonl')) return { value: options.transcript ?? '' }
    return { value: RUNNING }
  })
  on('http.fetch', (_$, e) => {
    attempts.fetches += 1
    // What the mod asks for, and not a report: kept apart so a count of reports holds.
    if (e.url.endsWith('/sessions/held')) {
      asked.push(JSON.parse(e.init?.body ?? '{}'))
      const messages = options.held?.splice(0) ?? []
      const commands = options.heldCommands?.splice(0) ?? []
      const answer = { value: { status: 200, ok: true, headers: {}, text: JSON.stringify({ messages, commands }) } }
      // Held open as Fleet holds an ask with nothing to hand over, until the test lets it go.
      return options.heldGate === undefined ? answer : options.heldGate.then(() => answer)
    }
    // A question held by Fleet answers when the test says; the terminal's settling is answered at once.
    if (e.url.endsWith('/sessions/ask/terminal')) {
      const body = JSON.parse(e.init?.body ?? '{}')
      questions.push(body)
      if (body.kind === 'asks') return { value: { status: 200, ok: true, headers: {}, text: '{"outcome":"asked","call":"helm-1"}' } }
      // Each poll takes the next answer the test queued: a string, a poll that never ends, or a failure.
      if (body.kind === 'wait') {
        const poll = options.polls?.shift() ?? new Promise<string>(() => undefined)
        if (poll instanceof Error) throw poll
        return Promise.resolve(poll).then(text => ({ value: { status: 200, ok: true, headers: {}, text } }))
      }
      return { value: { status: 200, ok: true, headers: {}, text: '{"outcome":"gone"}' } }
    }
    attempts.reports += 1
    // Fleet busy behind its store: the report is taken and not answered.
    if (options.slowReports) return new Promise(() => undefined)
    posts.push({ url: e.url, body: JSON.parse(e.init?.body ?? '{}') })
    return { value: { status: 200, ok: true, headers: {}, text: '{}' } }
  })
  on('process.run', (_$, e) => {
    if (e.argv[0] === 'git') {
      return { value: { exitCode: 0, stdout: 'fleet/session-ledger\n', stderr: '' } }
    }
    if (options.pullRequest !== undefined) return { value: { exitCode: 0, stdout: options.pullRequest, stderr: '' } }
    return { value: { exitCode: 1, stdout: '', stderr: 'no pull requests found' } }
  })
  on('session.id', () => ({ value: 'S1' }))
  on('session.model', () => ({ value: options.model?.() ?? 'claude-haiku-4-5-20251001' }))
  on('command.list', () => ({ value: [{ name: 'review', description: 'Review the pull request', source: 'builtin' }] }))
  on('command.run', (_$, e) => {
    ran.push({ command: e.command, args: e.args })
    options.onCommand?.()
    return { value: {} }
  })
  on('session.cwd', () => ({ value: '/repos/armada' }))
  // What the engine answers beneath the mod for the events a test raises.
  on('session.start', (_$, e) => ({ cwd: e.cwd }))
  on('session.end', (_$, e) => ({ sessionId: e.sessionId }))
  on('prompt.submit', (_$, e) => {
    submitted.push({ text: e.text, asUser: e.origin?.asUser })
    return { text: e.text }
  })
  on('turn.complete', () => ({ text: '' }))
  on('prompt.context', (_$, e) => ({ blocks: e.blocks }))
  on('classic.SessionStart', () => ({}))
  return { posts, attempts, clock, submitted, asked, ran, questions }
}

const facts = (posts: Posted[]) => posts.map(one => one.body.fact)
const kinds = (posts: Posted[]) => facts(posts).map(fact => fact.kind)

test('a session starting is told to Fleet with its directory and its branch', async ($, on) => {
  const { posts, clock } = world(on)
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()

  expect(posts[0].url).toBe('http://127.0.0.1:4242/sessions/report')
  expect(posts[0].body.harness).toBe('claude_code')
  expect(posts[0].body.session_id).toBe('S1')
  expect(posts[0].body.fact).toEqual({ kind: 'started', cwd: '/repos/armada', origin: 'terminal', mod_version: MOD_VERSION })
  expect(facts(posts)).toContainEqual({
    kind: 'attached',
    attachment: { kind: 'branch', target: 'fleet/session-ledger' },
  })
})

test('a Drone, Judge call or scout is not told to Fleet', async ($, on) => {
  const { posts, clock } = world(on, { env: { ARMADA_DRONE: '1' } })
  on('tool.call', { tool: 'Bash' }, () => ({ result: { stdout: URL, stderr: '', interrupted: false }, text: URL }))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.tool.call({ tool: 'Bash', command: 'gh pr create --base main' })
  await clock.settle()

  expect(posts).toEqual([])
})

test('a pull request a session opens is attached from the address gh printed', async ($, on) => {
  const { posts, clock } = world(on)
  on('tool.call', { tool: 'Bash' }, () => ({
    result: { stdout: URL, stderr: '', interrupted: false },
    text: URL,
  }))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.tool.call({ tool: 'Bash', command: 'gh pr create --base main' })
  await clock.settle()

  expect(facts(posts)).toContainEqual({
    kind: 'attached',
    attachment: { kind: 'pr', target: '1853', detail: { url: URL } },
  })
})

test('a pull request read off the branch carries its title, branch and address', async ($, on) => {
  const { posts, clock } = world(on, {
    pullRequest: JSON.stringify({ number: 1853, url: URL, state: 'OPEN', title: 'Pin the store clock', headRefName: 'fix/store-clock', isDraft: true }),
  })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()

  expect(facts(posts)).toContainEqual({
    kind: 'attached',
    attachment: {
      kind: 'pr',
      target: '1853',
      detail: { url: URL, address: URL, state: 'draft', title: 'Pin the store clock', branch: 'fix/store-clock' },
    },
  })
})

test('a message is reported as who it went to, and its text never leaves', async ($, on) => {
  const { posts, clock } = world(on)
  on('session.send', () => ({ isDelivered: true }))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.session.send({
    to: 'prod',
    text: 'the secret plan',
    origin: { kind: 'model' },
  })
  await clock.settle()

  expect(facts(posts)).toContainEqual({
    kind: 'attached',
    attachment: { kind: 'message', target: 'to:prod', detail: { direction: 'sent', count: '1' } },
  })
  expect(JSON.stringify(posts)).not.toContain('secret')
})

test('a Fleet that is not running costs nothing and is not tried again for a while', async ($, on) => {
  const { posts, attempts, clock } = world(on, { running: () => false })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()
  const tried = attempts.reads
  await $.turn.complete({
    answer: '',
    durationMs: 1,
    isAborted: false,
    turnId: 't1',
    reason: 'answer',
  })
  await clock.settle()

  expect(posts).toEqual([])
  expect(tried).toBeGreaterThan(0)
  expect(attempts.reads).toBe(tried)
})

test('what Fleet missed is told again when it answers, and nothing is lost to the silence', async ($, on) => {
  let up = false
  const { posts, clock } = world(on, { running: () => up })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()
  expect(posts).toEqual([])

  up = true
  await clock.advance(31_000)
  await $.prompt.submit({ text: 'fix the ledger', origin: { kind: 'user' } } as never)
  await clock.settle()

  expect(kinds(posts)).toContain('started')
  expect(facts(posts)).toContainEqual({ kind: 'titled', title: 'fix the ledger' })
})

test('a Fleet that is slow to answer is waited on for a moment and is not treated as gone', async ($, on) => {
  const { attempts, clock } = world(on, { slowReports: true })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.advance(2_000)
  const first = attempts.reports
  await $.turn.complete({ answer: '', durationMs: 1, isAborted: false, turnId: 't1', reason: 'answer' })
  await clock.advance(10_000)
  await clock.settle()

  expect(first).toBeGreaterThan(0)
  expect(attempts.reports).toBeGreaterThan(first)
})

test('the end of a session is told before the process goes', async ($, on) => {
  const { posts, clock } = world(on)
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  const ending = $.session.end({ reason: 'prompt_input_exit', sessionId: 'S1', resume: {} as never })
  await clock.settle()
  await ending

  expect(facts(posts).at(-1)).toEqual({ kind: 'ended', reason: 'prompt_input_exit' })
})

const ran = (command: string) => ({
  result: { stdout: 'ok', stderr: '', interrupted: false },
  text: 'ok',
})

const needs = (posts: Posted[]) =>
  facts(posts).filter(fact => JSON.stringify(fact).includes('"need"'))

test('declaring, taking and giving back a need are each reported as the session, and nothing else goes', async ($, on) => {
  const { posts, clock } = world(on)
  on('tool.call', { tool: 'Bash' }, e => ran(e.command))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()
  const before = posts.length

  await $.tool.call({
    tool: 'Bash',
    command: 'armada need ./protocol-version.toml "a minor, the secret plan"',
  })
  await $.tool.call({ tool: 'Bash', command: 'armada need --took protocol-version.toml "23.50"' })
  await $.tool.call({ tool: 'Bash', command: 'armada need --release protocol-version.toml' })
  await clock.settle()

  const sent = posts.slice(before)
  expect(sent.every(one => one.body.session_id === 'S1')).toBe(true)
  expect(facts(sent)).toEqual([
    {
      kind: 'attached',
      attachment: {
        kind: 'need',
        target: 'protocol-version.toml',
        detail: { what: 'a minor, the secret plan' },
      },
    },
    {
      kind: 'attached',
      attachment: {
        kind: 'need',
        target: 'protocol-version.toml',
        detail: { what: 'a minor, the secret plan', took: '23.50' },
      },
    },
    {
      kind: 'settled',
      attachment: { kind: 'need', target: 'protocol-version.toml' },
      state: 'given_back',
    },
  ])
})

test('a took with no declaration this session is not reported, and a status or other command is not either', async ($, on) => {
  const { posts, clock } = world(on)
  on('tool.call', { tool: 'Bash' }, e => ran(e.command))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.tool.call({ tool: 'Bash', command: 'armada need --took a.toml "1"' })
  await $.tool.call({ tool: 'Bash', command: 'armada need --status' })
  await $.tool.call({ tool: 'Bash', command: 'echo armada need a.toml "x"' })
  await clock.settle()

  expect(needs(posts)).toEqual([])
})

test('a need declared while Fleet is down costs nothing and is told again when it answers', async ($, on) => {
  let up = false
  const { posts, attempts, clock } = world(on, { running: () => up })
  on('tool.call', { tool: 'Bash' }, e => ran(e.command))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.tool.call({ tool: 'Bash', command: 'armada need a.toml "a minor"' })
  await clock.settle()
  expect(posts).toEqual([])
  expect(attempts.fetches).toBe(0)

  up = true
  await clock.advance(31_000)
  await $.prompt.submit({ text: 'go on', origin: { kind: 'user' } } as never)
  await clock.settle()
  expect(needs(posts)).toEqual([
    { kind: 'attached', attachment: { kind: 'need', target: 'a.toml', detail: { what: 'a minor' } } },
  ])
})

test('what a person sent from Bridge is submitted as their own prompt, once, in order', async ($, on) => {
  const { clock, submitted, asked } = world(on, { held: ['run the tests', 'then lint'] })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.advance(2000)
  await clock.settle()

  expect(asked[0]).toEqual({ session_id: 'S1', wait_ms: 25_000 })
  expect(submitted).toEqual([
    { text: 'run the tests', asUser: true },
    { text: 'then lint', asUser: true },
  ])
  await clock.advance(2000)
  await clock.settle()
  expect(submitted).toHaveLength(2)
})

test('a slow report does not silence the pickup of what a person sent', async ($, on) => {
  const { attempts, clock, submitted } = world(on, { slowReports: true, held: ['approved'] })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.advance(2_000)
  await clock.settle()

  expect(attempts.reports).toBeGreaterThan(0)
  expect(submitted).toEqual([{ text: 'approved', asUser: true }])
})

test('an ask held open by Fleet delivers the moment it answers, and only one is open at a time', async ($, on) => {
  let release = () => {}
  const heldGate = new Promise<void>(done => (release = done))
  const { clock, submitted, asked } = world(on, { held: ['go now'], heldGate })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.advance(2_000)
  await clock.advance(20_000)
  await clock.settle()
  expect(submitted).toEqual([])
  expect(asked).toHaveLength(1)

  release()
  await clock.settle()
  expect(submitted).toEqual([{ text: 'go now', asUser: true }])
})

test('a Fleet that answers an ask at once with nothing is asked no faster than every two seconds', async ($, on) => {
  const { clock, asked } = world(on)
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.advance(10_000)
  await clock.settle()

  expect(asked.length).toBeGreaterThan(1)
  expect(asked.length).toBeLessThanOrEqual(6)
})

test('a Fleet that is not running is not asked, and nothing is submitted', async ($, on) => {
  const { clock, submitted, asked } = world(on, { running: () => false, held: ['lost'] })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.advance(2000)
  await clock.settle()

  expect(asked).toEqual([])
  expect(submitted).toEqual([])
})

test('what the terminal runs on is told once with the commands it lists', async ($, on) => {
  const { posts, clock } = world(on)
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()

  expect(facts(posts)).toContainEqual({
    kind: 'tuned',
    model: 'haiku',
    commands: [{ name: 'review', says: 'Review the pull request' }],
    mod_version: MOD_VERSION,
  })
})

test('a model chosen in Bridge is run as a command, and the new one is told', async ($, on) => {
  let model = 'claude-haiku-4-5-20251001'
  const { posts, clock, ran, submitted } = world(on, {
    heldCommands: [{ command: 'model', args: 'sonnet' }],
    model: () => model,
    onCommand: () => {
      model = 'claude-sonnet-5-5'
    },
  })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.advance(2000)
  await clock.settle()

  expect(submitted).toEqual([])
  expect(facts(posts)).toContainEqual({ kind: 'tuned', model: 'sonnet' })
  expect(ran).toEqual([{ command: 'model', args: 'sonnet' }])
})

test('a rename in the terminal is told as the name, and the first prompt does not take it back', async ($, on) => {
  const { posts, clock } = world(on)
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.command.run({ command: 'rename', args: '  ledger   intake ' })
  await $.prompt.submit({ text: 'fix the ledger', origin: { kind: 'user' } } as never)
  await clock.settle()

  expect(facts(posts).filter(fact => fact.kind === 'titled')).toEqual([
    { kind: 'titled', title: 'ledger intake', named: true },
  ])
})

test('a bare rename is read from the transcript Claude Code wrote the name to', async ($, on) => {
  const transcript = '{"type":"custom-title","customTitle":"made up name","sessionId":"S1"}\n'
  const { posts, clock } = world(on, { transcript })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.command.run({ command: 'rename', args: '' })
  await clock.settle()

  expect(facts(posts)).toContainEqual({ kind: 'titled', title: 'made up name', named: true })
})

const artifacts = (posts: Posted[]) =>
  facts(posts).filter(fact => fact.kind === 'attached' && fact.attachment.kind === 'artifact')

test('a page, a new document and a Docs document are told as artifacts, and a code edit is not', async ($, on) => {
  const { posts, clock } = world(on)
  const page = 'https://claude.ai/artifact/p1'
  on('tool.call', (_$, e) => {
    if (e.tool === 'Artifact') return { result: {}, text: `Published ${page}` }
    if (e.tool === 'Write') {
      return { result: { type: String(e.file_path).endsWith('notes.md') ? 'update' : 'create' }, text: 'ok' }
    }
    return { result: {}, text: 'Created https://claude.ai/artifact/d1' }
  })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()
  const before = posts.length

  await $.tool.call({ tool: 'Artifact', file_path: '/repos/armada/spike.html', title: 'Spike' })
  await $.tool.call({ tool: 'Write', file_path: '/repos/armada/docs/spike.md', content: 'x' })
  await $.tool.call({ tool: 'Write', file_path: '/repos/armada/docs/notes.md', content: 'x' })
  await $.tool.call({ tool: 'Write', file_path: '/repos/armada/src/clock.ts', content: 'x' })
  await $.tool.call({
    tool: 'mcp__claude_ai_Claude_Docs__batch',
    container: { kind: 'project', create: { name: 'Write-up' } },
    batch: [],
  })
  await clock.settle()

  expect(artifacts(posts.slice(before)).map(fact => fact.attachment)).toEqual([
    { kind: 'artifact', target: page, detail: { form: 'page', title: 'Spike' } },
    {
      kind: 'artifact',
      target: '/repos/armada/docs/spike.md',
      detail: { form: 'file', title: 'spike.md' },
    },
    {
      kind: 'artifact',
      target: 'https://claude.ai/artifact/d1',
      detail: { form: 'doc', title: 'Write-up' },
    },
  ])
})

test('the model is told its Armada session id, and a Drone is not', async ($, on) => {
  world(on)
  const out = await $.prompt.context({ blocks: [] })
  expect(out.blocks.map(one => one.name)).toEqual(['armadaSession'])
  expect(out.blocks[0].text).toContain('S1')
  expect(out.blocks[0].text).toContain('waiting_for')
})

test('a Drone is told no session id', async ($, on) => {
  world(on, { env: { ARMADA_DRONE: '1' } })
  const out = await $.prompt.context({ blocks: [] })
  expect(out.blocks).toEqual([])
})

test('the session id is told again at every session start, and not to a Drone', async ($, on) => {
  world(on)
  for (const source of ['startup', 'resume', 'clear', 'compact'] as const) {
    const out = await $.classic.SessionStart({ source })
    expect(out.additionalContext?.[0]).toContain('S1')
  }
})

test('a Drone is told no session id at a start', async ($, on) => {
  world(on, { env: { ARMADA_DRONE: '1' } })
  const out = await $.classic.SessionStart({ source: 'startup' })
  expect(out.additionalContext ?? []).toEqual([])
})

test('an open of web pages is shown through Fleet and denied, not run', async ($, on) => {
  const { posts, clock } = world(on)
  let ranIt = false
  on('tool.call', { tool: 'Bash' }, () => {
    ranIt = true
    return { result: { stdout: '', stderr: '', interrupted: false }, text: '' }
  })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()
  const before = posts.length
  const out = await $.tool.call({
    tool: 'Bash',
    command: 'open "http://localhost:5191/?walk=a"; open -a Safari https://example.com/b',
  })
  await clock.settle()

  expect(ranIt).toBe(false)
  expect(JSON.stringify(out)).toContain('show_window')
  const shown = posts.slice(before).filter(one => one.url.endsWith('/sessions/window'))
  expect(shown.map(one => one.body)).toEqual([
    { url: 'http://localhost:5191/?walk=a', session_id: 'S1' },
    { url: 'https://example.com/b', session_id: 'S1' },
  ])
})

test('opening a file or a folder runs as usual', async ($, on) => {
  const { posts, clock } = world(on)
  let ranIt = 0
  on('tool.call', { tool: 'Bash' }, () => {
    ranIt += 1
    return { result: { stdout: '', stderr: '', interrupted: false }, text: '' }
  })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.tool.call({ tool: 'Bash', command: 'open file.txt' })
  await $.tool.call({ tool: 'Bash', command: 'open .' })
  await clock.settle()

  expect(ranIt).toBe(2)
  expect(posts.some(one => one.url.endsWith('/sessions/window'))).toBe(false)
})

test('a Drone may open what it likes', async ($, on) => {
  const { posts, clock } = world(on, { env: { ARMADA_DRONE: '1' } })
  let ranIt = false
  on('tool.call', { tool: 'Bash' }, () => {
    ranIt = true
    return { result: { stdout: '', stderr: '', interrupted: false }, text: '' }
  })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.tool.call({ tool: 'Bash', command: 'open http://localhost:5191/' })
  await clock.settle()

  expect(ranIt).toBe(true)
  expect(posts).toEqual([])
})

const SIZE = [
  {
    question: 'Which size?',
    header: 'Size',
    multiSelect: false,
    options: [
      { label: 'S', description: 'Small' },
      { label: 'L', description: 'Large' },
    ],
  },
]

test('a terminal question answered in Bridge is the tool result, and the terminal prompt is dropped', async ($, on) => {
  const { questions } = world(on, {
    polls: [JSON.stringify({ outcome: 'answered', updated_input: { questions: SIZE, answers: { 'Which size?': 'L' } } })],
  })
  // The terminal's own prompt, which nobody answers.
  on('tool.call', { tool: 'AskUserQuestion' }, () => new Promise(() => undefined))
  const out = await $.tool.call({ tool: 'AskUserQuestion', questions: SIZE })

  expect(out.result).toEqual({ questions: SIZE, answers: { 'Which size?': 'L' } })
  expect(questions).toEqual([
    { kind: 'asks', session_id: 'S1', input: { questions: SIZE } },
    { kind: 'wait', session_id: 'S1', call: 'helm-1' },
  ])
})

const ANSWERED = JSON.stringify({ outcome: 'answered', updated_input: { questions: SIZE, answers: { 'Which size?': 'L' } } })

test('a poll that times out or fails is asked again, and the answer is collected on a later one', async ($, on) => {
  const { questions, clock } = world(on, {
    // One that never comes back, one the connection dropped, one Fleet held and let go, then the answer.
    polls: [new Promise<string>(() => undefined), new Error('connection reset'), '{"outcome":"waiting"}', ANSWERED],
  })
  on('tool.call', { tool: 'AskUserQuestion' }, () => new Promise(() => undefined))
  const out = $.tool.call({ tool: 'AskUserQuestion', questions: SIZE })
  for (let i = 0; i < 6; i++) {
    await clock.advance(40_000)
    await clock.settle()
  }

  expect((await out).result).toEqual({ questions: SIZE, answers: { 'Which size?': 'L' } })
  expect(questions.filter(q => q.kind === 'wait')).toHaveLength(4)
  expect(questions.filter(q => q.kind === 'asks')).toHaveLength(1)
})

test('a Fleet that is down when the question is asked is waited out until it answers', async ($, on) => {
  let up = false
  const { questions, clock } = world(on, { running: () => up, polls: [ANSWERED] })
  on('tool.call', { tool: 'AskUserQuestion' }, () => new Promise(() => undefined))
  const out = $.tool.call({ tool: 'AskUserQuestion', questions: SIZE })
  await clock.advance(10_000)
  await clock.settle()
  expect(questions).toEqual([])

  up = true
  for (let i = 0; i < 3; i++) {
    await clock.advance(10_000)
    await clock.settle()
  }
  expect((await out).result).toEqual({ questions: SIZE, answers: { 'Which size?': 'L' } })
})

test('a terminal question refused in Bridge is told to the agent', async ($, on) => {
  world(on, { polls: [JSON.stringify({ outcome: 'refused', message: 'The person refused this.' })] })
  on('tool.call', { tool: 'AskUserQuestion' }, () => new Promise(() => undefined))
  const out = await $.tool.call({ tool: 'AskUserQuestion', questions: SIZE })

  expect(out.deny).toBe('The person refused this.')
})

test('a terminal question answered in the terminal first tells Fleet it was settled', async ($, on) => {
  const { questions, clock } = world(on)
  const typed = { questions: SIZE, answers: { 'Which size?': 'S' } }
  on('tool.call', { tool: 'AskUserQuestion' }, () => ({ result: typed, text: 'answered' }))
  const out = await $.tool.call({ tool: 'AskUserQuestion', questions: SIZE })
  await clock.settle()

  expect(out.result).toEqual(typed)
  expect(questions[0]).toEqual({ kind: 'asks', session_id: 'S1', input: { questions: SIZE } })
  expect(questions.at(-1)).toEqual({ kind: 'settled', session_id: 'S1', answered: true })
  expect(questions.filter(q => q.kind === 'wait')).toHaveLength(0)
})

test('a Drone does not put its question to Bridge', async ($, on) => {
  const { questions, clock } = world(on, { env: { ARMADA_DRONE: '1' } })
  on('tool.call', { tool: 'AskUserQuestion' }, () => ({ result: { questions: SIZE, answers: {} }, text: 'answered' }))
  await $.tool.call({ tool: 'AskUserQuestion', questions: SIZE })
  await clock.settle()

  expect(questions).toEqual([])
})
