import { expect, mock, test } from 'claude-code/testing'
import type { On } from 'claude-code'

// Fleet as the mod meets it: a runtime file naming a port, and whatever answers
// there. The engine's own `$` calls are what the test answers.
const RUNNING = '{"protocol_version":{"major":23,"minor":43},"pid":1,"port":4242,"started_at":"x"}'
const URL = 'https://github.com/Mele-Labs/armada/pull/1853'

type Posted = { url: string; body: any }

function world(on: On, options: { running?: () => boolean; hangs?: boolean } = {}) {
  const running = options.running ?? (() => true)
  const posts: Posted[] = []
  const attempts = { fetches: 0, reads: 0 }
  const clock = mock.clock(on, { now: 1_000_000 })
  mock.env(on, { HOME: '/home/u' })
  on('fs.read', () => {
    attempts.reads += 1
    if (!running()) throw new Error('no such file')
    return { value: RUNNING }
  })
  on('http.fetch', (_$, e) => {
    attempts.fetches += 1
    if (options.hangs) return new Promise(() => undefined)
    posts.push({ url: e.url, body: JSON.parse(e.init?.body ?? '{}') })
    return { value: { status: 200, ok: true, headers: {}, text: '{}' } }
  })
  on('process.run', (_$, e) => {
    if (e.argv[0] === 'git') {
      return { value: { exitCode: 0, stdout: 'fleet/session-ledger\n', stderr: '' } }
    }
    return { value: { exitCode: 1, stdout: '', stderr: 'no pull requests found' } }
  })
  on('session.id', () => ({ value: 'S1' }))
  on('session.cwd', () => ({ value: '/repos/armada' }))
  // What the engine answers beneath the mod for the events a test raises.
  on('session.start', (_$, e) => ({ cwd: e.cwd }))
  on('session.end', (_$, e) => ({ sessionId: e.sessionId }))
  on('prompt.submit', (_$, e) => ({ text: e.text }))
  on('turn.complete', () => ({ text: '' }))
  return { posts, attempts, clock }
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
  expect(posts[0].body.fact).toEqual({ kind: 'started', cwd: '/repos/armada', origin: 'terminal' })
  expect(facts(posts)).toContainEqual({
    kind: 'attached',
    attachment: { kind: 'branch', target: 'fleet/session-ledger' },
  })
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

test('a Fleet that never answers is waited on for a moment and then left alone', async ($, on) => {
  const { attempts, clock } = world(on, { hangs: true })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.advance(2_000)
  await $.session.end({ reason: 'other', sessionId: 'S1', resume: {} as never })
  await clock.advance(10_000)
  await clock.settle()

  expect(attempts.fetches).toBe(1)
})

test('the end of a session is told before the process goes', async ($, on) => {
  const { posts, clock } = world(on)
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  const ending = $.session.end({ reason: 'prompt_input_exit', sessionId: 'S1', resume: {} as never })
  await clock.settle()
  await ending

  expect(facts(posts).at(-1)).toEqual({ kind: 'ended', reason: 'prompt_input_exit' })
})
