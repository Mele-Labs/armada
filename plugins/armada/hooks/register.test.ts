import { expect, mock, test } from 'claude-code/testing'
import type { On } from 'claude-code'

import { MOD_VERSION } from './facts'
import { BASH_LAUNCH, NOTICE_COMPLETED, NOTICE_KILLED, NOTICE_SUBAGENT, TASK_STOPPED } from './background.fixture'

// Fleet as the mod meets it: a runtime file naming a port, and whatever answers
// there. The engine's own `$` calls are what the test answers.
const RUNNING = '{"protocol_id":"0000000000000000","pid":1,"port":4242,"started_at":"x"}'
const URL = 'https://github.com/Mele-Labs/armada/pull/1853'

type Posted = { url: string; body: any }

function world(
  on: On,
  options: { running?: () => boolean; slowReports?: boolean; heldGate?: Promise<void>; held?: string[]; heldCommands?: { command: string; args: string }[]; heldEnd?: boolean; model?: () => string; onCommand?: () => void; transcript?: string; tasksOnDisk?: Record<string, string>; env?: Record<string, string>; pullRequest?: string; cwd?: () => string; polls?: (string | Promise<string> | Error)[]; asks?: string; permission?: Record<string, unknown>; limits?: { kind: string; percentUsed: number; resetsAt?: string }[]; breakdown?: unknown; stopBlock?: string; thread?: unknown; logbook?: 'down' | 'slow' | 'refused' | string } = {},
) {
  const running = options.running ?? (() => true)
  const posts: Posted[] = []
  const attempts = { fetches: 0, reads: 0, reports: 0, listed: [] as string[] }
  const processes: { argv: string[] }[] = []
  const submitted: { text: string; asUser?: boolean }[] = []
  const usages: unknown[] = []
  const asked: unknown[] = []
  const logbookAsks: Posted[] = []
  const questions: any[] = []
  const ran: { command: string; args?: string }[] = []
  // What reached the engine, submits and commands alike, in the order they did.
  const order: string[] = []
  const clock = mock.clock(on, { now: 1_000_000 })
  mock.env(on, { HOME: '/home/user', ...options.env })
  // Claude Code's own task store: one file per task under the list's directory.
  on('fs.list', (_$, e) => {
    attempts.listed.push(e.path)
    if (options.tasksOnDisk === undefined || !e.path.includes('/.claude/tasks/')) throw new Error('no such directory')
    return { value: Object.keys(options.tasksOnDisk).map(name => ({ name, kind: 'file' as const, size: 1, mtimeMs: 0, isLink: false })) }
  })
  on('fs.read', (_$, e) => {
    if (e.path.includes('/.claude/tasks/')) {
      const text = options.tasksOnDisk?.[e.path.split('/').at(-1) ?? '']
      if (text === undefined) throw new Error('no such file')
      return { value: text }
    }
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
      // Handed over once, as Fleet does.
      const end = options.heldEnd === true ? true : undefined
      options.heldEnd = false
      const answer = { value: { status: 200, ok: true, headers: {}, text: JSON.stringify({ messages, commands, end }) } }
      // Held open as Fleet holds an ask with nothing to hand over, until the test lets it go.
      return options.heldGate === undefined ? answer : options.heldGate.then(() => answer)
    }
    // A question held by Fleet answers when the test says; the terminal's settling is answered at once.
    if (e.url.endsWith('/sessions/ask/terminal')) {
      const body = JSON.parse(e.init?.body ?? '{}')
      questions.push(body)
      if (body.kind === 'asks') return { value: { status: 200, ok: true, headers: {}, text: options.asks ?? '{"outcome":"asked","call":"helm-1"}' } }
      // Each poll takes the next answer the test queued: a string, a poll that never ends, or a failure.
      if (body.kind === 'wait') {
        const poll = options.polls?.shift() ?? new Promise<string>(() => undefined)
        if (poll instanceof Error) throw poll
        return Promise.resolve(poll).then(text => ({ value: { status: 200, ok: true, headers: {}, text } }))
      }
      return { value: { status: 200, ok: true, headers: {}, text: '{"outcome":"gone"}' } }
    }
    // What the Logbook hands a subagent: its text, a Fleet that fails, one too slow to wait for, or one without the route.
    if (e.url.endsWith('/logbook/subagent')) {
      logbookAsks.push({ url: e.url, body: JSON.parse(e.init?.body ?? '{}') })
      if (options.logbook === 'down') throw new Error('connection refused')
      if (options.logbook === 'slow') return new Promise(() => undefined)
      if (options.logbook === 'refused') return { value: { status: 404, ok: false, headers: {}, text: '{}' } }
      return { value: { status: 200, ok: true, headers: {}, text: JSON.stringify({ text: options.logbook ?? '' }) } }
    }
    // The session as Fleet holds it, or a Fleet that does not know it.
    if (e.url.includes('/sessions/one?')) {
      if (options.thread === undefined) return { value: { status: 404, ok: false, headers: {}, text: '{}' } }
      return { value: { status: 200, ok: true, headers: {}, text: JSON.stringify(options.thread) } }
    }
    attempts.reports += 1
    // Fleet busy behind its store: the report is taken and not answered.
    if (options.slowReports) return new Promise(() => undefined)
    posts.push({ url: e.url, body: JSON.parse(e.init?.body ?? '{}') })
    return { value: { status: 200, ok: true, headers: {}, text: '{}' } }
  })
  on('process.run', (_$, e) => {
    processes.push(e)
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
    order.push(`run /${e.command}`)
    options.onCommand?.()
    return { value: {} }
  })
  on('session.cwd', () => ({ value: options.cwd?.() ?? '/repos/armada' }))
  on('session.root', () => ({ value: '/repos/armada' }))
  // What the engine answers beneath the mod for the events a test raises.
  on('session.start', (_$, e) => ({ cwd: e.cwd }))
  on('session.end', (_$, e) => ({ sessionId: e.sessionId }))
  on('prompt.submit', (_$, e) => {
    submitted.push({ text: e.text, asUser: e.origin?.asUser })
    order.push(`submit ${e.text}`)
    return { text: e.text }
  })
  on('turn.complete', () => ({ text: '' }))
  on('session.measure', (_$, e) => ({ changed: e.changed }))
  on('classic.StopFailure', () => ({}))
  on('classic.PermissionRequest', () => options.permission ?? {})
  on('session.usage', (_$, e) => {
    usages.push(e.breakdown)
    return {
      value: {
        startedAt: 0,
        context: { window: 200_000, ...(e.breakdown === undefined ? {} : { breakdown: options.breakdown }) },
        rateLimits: options.limits ?? [],
      },
    }
  })
  on('prompt.context', (_$, e) => ({ blocks: e.blocks }))
  on('prompt.compose', () => ({ sections: [] }))
  on('classic.SessionStart', () => ({}))
  on('classic.Stop', () => (options.stopBlock === undefined ? {} : { block: options.stopBlock }))
  on('tool.call', { tool: 'mcp__armada-fleet__waiting_for' }, () => ({ result: {}, text: '{}' }))
  return { posts, attempts, clock, submitted, asked, ran, questions, order, usages, processes, logbookAsks }
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

test('the branch and its pull request are read where the session is now, not where it last said it was', async ($, on) => {
  let here = '/repos/armada'
  const { processes, clock } = world(on, { cwd: () => here })
  on('tool.call', { tool: 'Bash' }, e => ran(e.command))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()
  processes.length = 0

  // The shell is in a slot now, and the cached directory still names the first.
  here = '/repos/armada/.armada/slots/slot-13'
  await $.tool.call({ tool: 'Bash', command: 'git checkout topic' })
  await clock.settle()

  expect(processes.length).toBeGreaterThan(0)
  for (const one of processes) {
    expect(JSON.stringify(one)).toContain('/repos/armada/.armada/slots/slot-13')
  }
})

test('a Drone, Judge call or scout is not told to Fleet', async ($, on) => {
  const { posts, clock } = world(on, { env: { ARMADA_DRONE: '1' } })
  on('tool.call', { tool: 'Bash' }, () => ({ result: { stdout: URL, stderr: '', interrupted: false }, text: URL }))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.tool.call({ tool: 'Bash', command: 'gh pr create --base main' })
  await $.session.end({ reason: 'other', sessionId: 'S1', resume: {} as never })
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
    attachment: { kind: 'pr', target: '1853', detail: { url: URL, created: 'true' } },
  })
})

test('an issue a session files is attached from the address gh printed', async ($, on) => {
  const { posts, clock } = world(on)
  const ISSUE = 'https://github.com/Mele-Labs/armada/issues/2214'
  on('tool.call', { tool: 'Bash' }, () => ({
    result: { stdout: `Creating issue in Mele-Labs/armada\n\n${ISSUE}`, stderr: '', interrupted: false },
    text: `Creating issue in Mele-Labs/armada\n\n${ISSUE}`,
  }))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.tool.call({ tool: 'Bash', command: 'gh issue create --title x --body-file b.md' })
  await $.tool.call({ tool: 'Bash', command: 'gh issue view 2214' })
  await clock.settle()

  const told = facts(posts).filter(one => one.kind === 'attached' && one.attachment.kind === 'issue')
  expect(told).toEqual([{ kind: 'attached', attachment: { kind: 'issue', target: '2214', detail: { address: ISSUE } } }])
})

test('a pull request a session opens is read by its address, so it carries its title from any slot', async ($, on) => {
  const { posts, processes, clock } = world(on, {
    cwd: () => '/repos/armada/.armada/slots/slot-2',
    pullRequest: JSON.stringify({ number: 1853, url: URL, state: 'OPEN', title: 'Pin the store clock', headRefName: 'fix/store-clock' }),
  })
  on('tool.call', { tool: 'Bash' }, () => ({
    result: { stdout: URL, stderr: '', interrupted: false },
    text: URL,
  }))
  await $.session.start({ cwd: '/repos/armada/.armada/slots/slot-2', surface: null, isInteractive: false })
  await $.tool.call({ tool: 'Bash', command: 'cd /repos/armada/.armada/slots/slot-4 && gh pr create --base main' })
  await clock.settle()

  expect(processes.some(one => one.argv.join(' ').includes(`gh pr view ${URL}`))).toBe(true)
  expect(facts(posts)).toContainEqual({
    kind: 'attached',
    attachment: {
      kind: 'pr',
      target: '1853',
      detail: { url: URL, address: URL, state: 'open', title: 'Pin the store clock', branch: 'fix/store-clock', created: 'true' },
    },
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

test('a subagent is attached by its agent id and names the call that started it', async ($, on) => {
  const { posts, clock } = world(on)
  on('agent.spawn', () => ({ model: 'claude-haiku-4-5', agentId: 'a4c6' }))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.agent.spawn({ prompt: 'x', description: 'Mock walk', subagentType: 'bridge-engineer', tool_use_id: 'toolu_1' } as never)
  await clock.settle()

  expect(facts(posts)).toContainEqual({
    kind: 'attached',
    attachment: { kind: 'subagent', target: 'a4c6', detail: { type: 'bridge-engineer', description: 'Mock walk', call: 'toolu_1' } },
  })
})

// What the subagent runs with is what the hook beneath the mod is handed.
async function spawnedWith(on: On, $: any, prompt: string, options: Parameters<typeof world>[1]) {
  const { posts, clock, logbookAsks } = world(on, options)
  const prompts: string[] = []
  on('agent.spawn', (_$, e) => {
    prompts.push(e.prompt)
    return { model: 'claude-haiku-4-5', agentId: 'a4c6' }
  })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  const spawning = $.agent.spawn({ prompt, description: 'Mock walk', subagentType: 'bridge-engineer', tool_use_id: 'toolu_1' } as never)
  // Past the wait for the Logbook, which a Fleet too slow to answer runs out.
  await clock.advance(2000)
  const out = await spawning
  await clock.settle()
  return { prompts, posts, out, logbookAsks }
}

test('a subagent is given the Logbook lines Fleet has for the paths its prompt names', async ($, on) => {
  const lines = 'How the owner works:\n- No AI-sounding copy.'
  const { prompts, logbookAsks, out } = await spawnedWith(on, $, 'Fix apps/desktop/src/main.ts', { logbook: lines })

  expect(logbookAsks.map(one => one.body)).toEqual([{ session_id: 'S1', prompt: 'Fix apps/desktop/src/main.ts' }])
  expect(prompts).toEqual([`Fix apps/desktop/src/main.ts\n\n${lines}`])
  expect(out.agentId).toBe('a4c6')
})

for (const logbook of ['down', 'slow', 'refused', '']) {
  test(`a Fleet that answers ${logbook === '' ? 'nothing' : logbook} gives the subagent its prompt as it was, and no error`, async ($, on) => {
    const { prompts, out } = await spawnedWith(on, $, 'Fix apps/desktop/src/main.ts', { logbook })

    expect(prompts).toEqual(['Fix apps/desktop/src/main.ts'])
    expect(out.agentId).toBe('a4c6')
  })
}

test('a Fleet that is not running is not asked for the Logbook', async ($, on) => {
  const { prompts, logbookAsks } = await spawnedWith(on, $, 'Fix apps/desktop/src/main.ts', { running: () => false, logbook: 'text' })

  expect(prompts).toEqual(['Fix apps/desktop/src/main.ts'])
  expect(logbookAsks).toEqual([])
})

test('a Drone gives its subagents no Logbook', async ($, on) => {
  const { prompts, logbookAsks } = await spawnedWith(on, $, 'Fix it', { env: { ARMADA_DRONE: '1' }, logbook: 'text' })

  expect(prompts).toEqual(['Fix it'])
  expect(logbookAsks).toEqual([])
})

test('a subagent that stops is settled as spent', async ($, on) => {
  const { posts, clock } = world(on)
  on('classic.SubagentStop', () => ({}))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.classic.SubagentStop({ stop_hook_active: false, agent_id: 'a1b2', agent_transcript_path: '/t.jsonl', agent_type: 'Explore' })
  await clock.settle()

  expect(facts(posts)).toContainEqual({ kind: 'settled', attachment: { kind: 'subagent', target: 'a1b2' }, state: 'spent' })
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

test('End session pressed in Bridge runs /exit once, after the messages held with it', async ($, on) => {
  const { clock, order, ran } = world(on, { held: ['one last thing'], heldEnd: true })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.advance(2000)
  await clock.settle()

  expect(order).toEqual(['submit one last thing', 'run /exit'])
  await clock.advance(4000)
  await clock.settle()
  expect(ran.filter(one => one.command === 'exit')).toHaveLength(1)
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

const background = (posts: Posted[]) =>
  facts(posts).filter(fact => (fact.kind === 'attached' || fact.kind === 'settled') && fact.attachment.kind === 'background')

test('a command run in the background and a Monitor are told, and settled on the notice or a stop, never on a subagent\'s', async ($, on) => {
  const { posts, clock } = world(on)
  let launches = 0
  on('tool.call', (_$, e) => {
    if (e.tool === 'Bash' && e.run_in_background === true) return { result: {}, text: BASH_LAUNCH.replaceAll('b3wd556q5', `b${++launches}wd556q5`) }
    if (e.tool === 'Monitor') return { result: {}, text: 'Monitor started (task mon42, timeout 300000ms).' }
    if (e.tool === 'TaskStop') return { result: {}, text: TASK_STOPPED }
    return { result: {}, text: 'ok' }
  })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()
  const before = posts.length

  // Three in the background; the notices name the engine's ids, so the launches are given theirs.
  await $.tool.call({ tool: 'Bash', command: 'pnpm build', description: 'Build the renderer', run_in_background: true })
  await $.tool.call({ tool: 'Bash', command: 'git status', description: 'Show status' })
  await $.tool.call({ tool: 'Monitor', command: 'gh run watch 9', description: 'Watch CI', timeout_ms: 60000 })
  await $.tool.call({ tool: 'Bash', command: 'pnpm dev', description: 'Serve it', run_in_background: true })
  await clock.settle()
  const ended = (text: string) => $.prompt.submit({ text, origin: { kind: 'task-notification' } } as never)
  // Inside a system-reminder, under an origin that is not a task notification's.
  await $.prompt.submit({ text: NOTICE_COMPLETED.replace('byry3zett', 'b1wd556q5'), origin: { kind: 'unclassified' } } as never)
  await ended(NOTICE_SUBAGENT)
  await $.prompt.submit({ text: 'hello', origin: { kind: 'user' } } as never)
  await ended(NOTICE_KILLED.replace('bdjviwn7c', 'b2wd556q5'))
  await $.tool.call({ tool: 'TaskStop', task_id: 'mon42' })
  await clock.settle()

  const path = (id: string) => `/private/tmp/claude-501/-Users-nickmele-Development-armada--armada-slots-slot-12/aa994f6f-56ce-4b09-9e06-66342d14548f/tasks/${id}wd556q5.output`
  const build = { form: 'command', task: 'Build the renderer', command: 'pnpm build', output: path('b1') }
  const serve = { form: 'command', task: 'Serve it', command: 'pnpm dev', output: path('b2') }
  const watch = { form: 'monitor', task: 'Watch CI', command: 'gh run watch 9' }
  expect(background(posts.slice(before)).map(fact => (fact.kind === 'attached' ? fact.attachment : [fact.attachment, fact.state]))).toEqual([
    { kind: 'background', target: 'b1wd556q5', detail: build },
    { kind: 'background', target: 'mon42', detail: watch },
    { kind: 'background', target: 'b2wd556q5', detail: serve },
    // The engine's own file replaces the one parsed from the launch.
    { kind: 'background', target: 'b1wd556q5', detail: { ...build, status: 'completed', exit: '0', output: '/private/tmp/claude-501/.../tasks/byry3zett.output' } },
    [{ kind: 'background', target: 'b1wd556q5' }, 'spent'],
    { kind: 'background', target: 'b2wd556q5', detail: { ...serve, status: 'killed', output: '/private/tmp/claude-501/.../tasks/bdjviwn7c.output' } },
    [{ kind: 'background', target: 'b2wd556q5' }, 'spent'],
    { kind: 'background', target: 'mon42', detail: { ...watch, status: 'killed' } },
    [{ kind: 'background', target: 'mon42' }, 'spent'],
  ])
})

test('a file written is told as its path from the session directory, and never its content', async ($, on) => {
  const { posts, clock } = world(on)
  on('tool.call', () => ({ result: {}, text: 'ok' }))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()
  const before = posts.length

  await $.tool.call({ tool: 'Edit', file_path: '/repos/armada/src/a.rs', old_string: 'secret', new_string: 'more' })
  await $.tool.call({ tool: 'MultiEdit', file_path: '/repos/armada/src/b.rs', edits: [] })
  await $.tool.call({ tool: 'Write', file_path: '/etc/hosts', content: 'x' })
  await clock.settle()

  const touched = facts(posts.slice(before)).filter(fact => fact.kind === 'touched')
  expect(touched).toEqual([
    { kind: 'touched', paths: ['src/a.rs'], act: 'edit' },
    { kind: 'touched', paths: ['src/b.rs'], act: 'edit' },
  ])
  expect(JSON.stringify(posts)).not.toContain('secret')
})

test('a file read and a command run are told as a path and a first line, and not the content', async ($, on) => {
  const { posts, clock } = world(on)
  on('tool.call', () => ({ result: {}, text: 'file contents nobody sends' }))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()
  const before = posts.length

  await $.tool.call({ tool: 'Read', file_path: '/repos/armada/src/a.rs' })
  await $.tool.call({ tool: 'Read', file_path: '/etc/hosts' })
  await $.tool.call({ tool: 'Bash', command: 'cargo test --workspace\nsecond line' })
  await clock.settle()

  const told = facts(posts.slice(before)).filter(fact => fact.kind === 'read' || fact.kind === 'ran')
  expect(told).toEqual([
    { kind: 'read', path: 'src/a.rs' },
    { kind: 'ran', command: 'cargo test --workspace' },
  ])
  expect(JSON.stringify(posts)).not.toContain('nobody sends')
  expect(JSON.stringify(posts)).not.toContain('second line')
})

test('the model is told its Armada job id, and a Drone is not', async ($, on) => {
  world(on)
  const out = await $.prompt.context({ blocks: [] })
  expect(out.blocks.map(one => one.name)).toEqual(['armadaSession'])
  expect(out.blocks[0].text).toContain('S1')
  expect(out.blocks[0].text).toContain('waiting_for')
  expect(out.blocks[0].text).toContain('Armada keeps the question, whole, in his Needs you list')
  expect(out.blocks[0].text).toContain('already waits on him by itself, so never put it in waiting_for')
  expect(out.blocks[0].text).not.toContain('a page to look at')
  expect(out.blocks[0].text).toContain('call all_done')
  expect(out.blocks[0].text).toContain('call finish_up')
  expect(out.blocks[0].text).toContain('A pull request in the merge queue or with auto-merge on is not done')
  expect(out.blocks[0].text).toContain('never put a wait on it in waiting_for')
  expect(out.blocks[0].text).toContain('passes steps, a list of short lines')
  expect(out.blocks[0].text).toContain('never in an Actions table')
  expect(out.blocks[0].text).toContain('one way only')
  expect(out.blocks[0].text).toContain('fenced block tagged bash')
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

test('the first-message note says a question is kept whole for him during a walk, and one about the page is asked in the window', async ($, on) => {
  world(on)
  const out = await $.classic.SessionStart({ source: 'startup' })
  const note = out.additionalContext?.[0] ?? ''
  expect(note).toContain('never add it to waiting_for')
  expect(note).toContain('call ask_in_window')
})

test('the first-message note names show_sketch for any diagram and forbids ASCII art', async ($, on) => {
  world(on)
  const out = await $.classic.SessionStart({ source: 'startup' })
  const note = out.additionalContext?.[0] ?? ''
  expect(note).toContain('call show_sketch')
  expect(note).toContain('never draw ASCII art')
})

test('the first-message note names show_file for putting a file in front of him', async ($, on) => {
  world(on)
  const out = await $.classic.SessionStart({ source: 'startup' })
  const note = out.additionalContext?.[0] ?? ''
  expect(note).toContain('call show_file')
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

test('a file the session itself writes is told, and one a subagent writes is not', async ($, on) => {
  const { posts, clock } = world(on)
  on('tool.call', () => ({ result: {}, text: 'ok' }))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()
  const before = posts.length

  await $.tool.call({ tool: 'Edit', file_path: '/repos/armada/src/clock.ts', old_string: 'a', new_string: 'b' })
  await $.tool.call({ tool: 'Edit', file_path: '/repos/armada/src/clock.ts', old_string: 'b', new_string: 'c', agentId: 'sub1' })
  await $.tool.call({ tool: 'NotebookEdit', notebook_path: '/repos/armada/n.ipynb', new_source: 'x' })
  await clock.settle()

  expect(kinds(posts.slice(before)).filter(kind => kind === 'wrote')).toHaveLength(2)
})

test('a file the session writes outside its folder, such as a memory note, is not told', async ($, on) => {
  const { posts, clock } = world(on)
  on('tool.call', () => ({ result: {}, text: 'ok' }))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()
  const before = posts.length

  await $.tool.call({ tool: 'Write', file_path: '/Users/me/.claude/projects/x/memory/MEMORY.md', content: 'a' })
  await $.tool.call({ tool: 'Edit', file_path: '/repos/armada-other/src/a.ts', old_string: 'a', new_string: 'b' })
  await clock.settle()

  expect(kinds(posts.slice(before)).filter(kind => kind === 'wrote')).toHaveLength(0)
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

test('a terminal question Fleet refuses while a walk is open is denied before the terminal prompt opens', async ($, on) => {
  const refused = 'Refused: the owner is walking a page you showed him. Your question is kept in his Needs you list. End your turn.'
  const { questions } = world(on, { asks: JSON.stringify({ outcome: 'refused', message: refused }) })
  let prompted = false
  on('tool.call', { tool: 'AskUserQuestion' }, () => {
    prompted = true
    return new Promise(() => undefined)
  })
  const out = await $.tool.call({ tool: 'AskUserQuestion', questions: SIZE })

  expect(out.deny).toBe(refused)
  expect(prompted).toBe(false)
  expect(questions).toEqual([{ kind: 'asks', session_id: 'S1', input: { questions: SIZE } }])
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
  // The call is known before the prompt opens, so one poll may already be out when it is answered.
  expect(questions.filter(q => q.kind === 'wait').length).toBeLessThanOrEqual(1)
})

test('a Drone does not put its question to Bridge', async ($, on) => {
  const { questions, clock } = world(on, { env: { ARMADA_DRONE: '1' } })
  on('tool.call', { tool: 'AskUserQuestion' }, () => ({ result: { questions: SIZE, answers: {} }, text: 'answered' }))
  await $.tool.call({ tool: 'AskUserQuestion', questions: SIZE })
  await clock.settle()

  expect(questions).toEqual([])
})

test('a measurement carries the plan windows, and a window that moved is not held back by the throttle', async ($, on) => {
  const { posts, clock } = world(on)
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  const five = { kind: 'five_hour', percentUsed: 41.4, resetsAt: '2026-10-12T22:40:00Z' }
  const context = { tokens: 1000, window: 200_000 }
  await $.session.measure({ context, rateLimits: [five], changed: ['context', 'rateLimits'] })
  // Inside the ten seconds, and only the context moved: held back.
  await $.session.measure({ context: { ...context, tokens: 2000 }, rateLimits: [five], changed: ['context'] })
  // Inside the ten seconds, and the window moved: told.
  await $.session.measure({ context, rateLimits: [{ ...five, percentUsed: 42 }], changed: ['rateLimits'] })
  await clock.settle()

  const measured = facts(posts).filter(fact => fact.kind === 'measured')
  expect(measured.map(fact => fact.plan)).toEqual([
    [{ kind: 'five_hour', percent_used: 41, resets_at: '2026-10-12T22:40:00.000Z' }],
    [{ kind: 'five_hour', percent_used: 42, resets_at: '2026-10-12T22:40:00.000Z' }],
  ])
})

test('a reading the throttle held back is told once the ten seconds are up, and the newest of them', async ($, on) => {
  const { posts, clock } = world(on)
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  const reading = (tokens: number) => ({ context: { tokens, window: 200_000 }, rateLimits: [], changed: ['context' as const] })
  await $.session.measure(reading(1000))
  await clock.advance(3000)
  await $.session.measure(reading(2000))
  await $.session.measure(reading(3000))
  await clock.settle()
  const told = () => facts(posts).filter(fact => fact.kind === 'measured').map(fact => fact.usage.context_tokens)
  expect(told()).toEqual([1000])

  await clock.advance(7000)
  expect(told()).toEqual([1000, 3000])
})

// A turn's own measurement comes at its end, so a long turn is read from each of its model requests.
test('a model request on the main thread is measured while its turn runs, and a subagent\'s is not', async ($, on) => {
  const { posts, clock } = world(on)
  on('turn.step', async function* (_$, e) {
    return {
      turnId: e.turnId,
      index: e.index,
      answer: '',
      toolUses: [],
      stopReason: 'tool_use' as const,
      usage: { input_tokens: 10, cache_creation_input_tokens: 20, cache_read_input_tokens: 90_000, output_tokens: 5, model: 'm' },
    }
  })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  const step = async (agentId?: string) => {
    const stream = $.turn.step({ turnId: 't1', index: 0, model: 'm', messageCount: 1, ...(agentId === undefined ? {} : { agentId }) })
    for await (const _ of stream) void _
    await stream.result
  }
  await step('a1')
  await clock.advance(10_000)
  expect(facts(posts).filter(fact => fact.kind === 'measured')).toEqual([])

  await step()
  await clock.settle()
  const measured = facts(posts).filter(fact => fact.kind === 'measured')
  expect(measured.map(fact => fact.usage.context_tokens)).toEqual([90_030])
  expect('context_breakdown' in measured[0]).toBe(false)
})

test('a measurement off a subscription names no plan', async ($, on) => {
  const { posts, clock } = world(on)
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.session.measure({ context: { tokens: 1000, window: 200_000 }, rateLimits: [], changed: ['context'] })
  await clock.settle()

  const measured = facts(posts).find(fact => fact.kind === 'measured')
  expect(measured).toBeDefined()
  expect('plan' in measured).toBe(false)
})

test('a turn that ends is told with the close of what it said, and the agent is never asked', async ($, on) => {
  const { posts, clock } = world(on)
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.turn.complete({ answer: 'The cap is five.\nThe backoff stops there.', durationMs: 1, isAborted: false, turnId: 't1', reason: 'answer' })
  await clock.settle()

  expect(facts(posts)).toContainEqual({ kind: 'turn_completed', said: 'The cap is five.\nThe backoff stops there.' })
})

test('a turn that said nothing is told as over and carries no line', async ($, on) => {
  const { posts, clock } = world(on)
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.turn.complete({ answer: '  ', durationMs: 1, isAborted: false, turnId: 't1', reason: 'answer' })
  await clock.settle()

  expect(facts(posts)).toContainEqual({ kind: 'turn_completed' })
})

test('a prompt going in tells Fleet a turn began, and a slash command is not a turn', async ($, on) => {
  const { posts, clock } = world(on)
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.prompt.submit({ text: '/compact', origin: { kind: 'user' } } as never)
  await clock.settle()
  expect(kinds(posts)).not.toContain('turn_started')

  await $.prompt.submit({ text: 'fix the ledger', origin: { kind: 'user' } } as never)
  await clock.settle()
  expect(facts(posts)).toContainEqual({ kind: 'turn_started' })
})

test('a turn that failed on the plan limit tells Fleet when the full window opens, and is not a finished turn', async ($, on) => {
  const { posts, clock } = world(on, {
    limits: [
      { kind: 'five_hour', percentUsed: 100, resetsAt: '2026-10-12T22:40:00Z' },
      { kind: 'seven_day', percentUsed: 30, resetsAt: '2026-10-17T09:00:00Z' },
    ],
  })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.turn.complete({ answer: '', durationMs: 1, isAborted: false, turnId: 't1', reason: 'error' })
  await $.classic.StopFailure({ error: 'rate_limit' })
  await clock.settle()

  expect(facts(posts)).toContainEqual({ kind: 'limited', resets_at: '2026-10-12T22:40:00.000Z' })
  expect(kinds(posts)).not.toContain('turn_completed')
})

test('a failure that is not the plan limit, or a limit no window shows full, is not told', async ($, on) => {
  const { posts, clock } = world(on, { limits: [{ kind: 'five_hour', percentUsed: 60, resetsAt: '2026-10-12T22:40:00Z' }] })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.classic.StopFailure({ error: 'overloaded' })
  await $.classic.StopFailure({ error: 'rate_limit' })
  await clock.settle()

  expect(kinds(posts)).not.toContain('limited')
})

test('a Drone that fails on the plan limit tells Fleet nothing', async ($, on) => {
  const { posts, clock } = world(on, {
    env: { ARMADA_DRONE: '1' },
    limits: [{ kind: 'five_hour', percentUsed: 100, resetsAt: '2026-10-12T22:40:00Z' }],
  })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.classic.StopFailure({ error: 'rate_limit' })
  await clock.settle()

  expect(posts).toEqual([])
})

test('a turn that failed on a sign-in that ran out tells Fleet it signed out, and is not a finished turn', async ($, on) => {
  const { posts, clock } = world(on)
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.turn.complete({ answer: '', durationMs: 1, isAborted: false, turnId: 't1', reason: 'error' })
  await $.classic.StopFailure({ error: 'authentication_failed' })
  await clock.settle()

  expect(facts(posts)).toContainEqual({ kind: 'signed_out' })
  expect(kinds(posts)).not.toContain('turn_completed')
})

test('a Drone whose sign-in ran out tells Fleet nothing, since Fleet reads its own', async ($, on) => {
  const { posts, clock } = world(on, { env: { ARMADA_DRONE: '1' } })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.classic.StopFailure({ error: 'authentication_failed' })
  await clock.settle()

  expect(posts).toEqual([])
})

test('what fills the context window is told as names, paths and counts, ten tools at most, and only as the context moves', async ($, on) => {
  const tools = Array.from({ length: 12 }, (_, at) => ({
    name: `mcp__srv__tool${at}`,
    serverName: 'srv',
    tokens: 100 + at,
    isLoaded: true,
  }))
  const breakdown = {
    categories: [
      { name: 'System prompt', tokens: 3000, color: 'promptBorder', isDeferred: false, kind: 'used' },
      { name: 'Free space', tokens: 150_000, color: 'inactive', isDeferred: false, kind: 'free' },
    ],
    totalTokens: 50_000,
    maxTokens: 200_000,
    rawMaxTokens: 200_000,
    gridRows: [[{ color: 'x', isFilled: true, categoryName: 'Messages', tokens: 9, percentage: 1, squareFullness: 1 }]],
    memoryFiles: [{ path: '/repo/CLAUDE.md', type: 'Project', tokens: 800 }],
    mcpTools: tools,
    skills: { totalTokens: 5, skills: [{ name: 'a secret skill', source: 'plugin', tokens: 5 }] },
    autoCompactThreshold: 160_000,
    isAutoCompactEnabled: true,
    apiUsage: null,
  }
  const { posts, usages, clock } = world(on, { breakdown })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.session.measure({ context: { tokens: 1000, window: 200_000 }, rateLimits: [], changed: ['context'] })
  // Only the cost moved, inside the throttle: nothing is asked of the engine or told.
  await $.session.measure({ context: { tokens: 1000, window: 200_000 }, rateLimits: [], changed: ['cost'] })
  await clock.settle()

  // One for the Loadout at the start, one for the measurement whose context moved.
  expect(usages).toEqual(['summary', 'summary'])
  const told = facts(posts).find(fact => fact.kind === 'measured').context_breakdown
  expect(told).toEqual({
    categories: [
      { name: 'System prompt', tokens: 3000, kind: 'used' },
      { name: 'Free space', tokens: 150_000, kind: 'free' },
    ],
    memory_files: [{ path: '/repo/CLAUDE.md', tokens: 800 }],
    mcp_tools: tools
      .slice(2)
      .reverse()
      .map(one => ({ name: one.name, server: 'srv', tokens: one.tokens })),
    total_tokens: 50_000,
    max_tokens: 200_000,
    autocompact_threshold: 160_000,
  })
  expect(JSON.stringify(posts)).not.toContain('secret')
})

test('a measurement whose context did not move asks the engine for no breakdown', async ($, on) => {
  const { posts, usages, clock } = world(on)
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.session.measure({ context: { tokens: 1000, window: 200_000 }, rateLimits: [], changed: ['cost'] })
  await clock.settle()

  // The Loadout's own reading at the start, and none for the measurement.
  expect(usages).toEqual(['summary'])
  expect('context_breakdown' in facts(posts).find(fact => fact.kind === 'measured')).toBe(false)
})

const tasksTold = (posts: Posted[]) => facts(posts).filter(fact => fact.kind === 'tasks').map(fact => fact.tasks)

test('the agent\'s task list is told whole by TodoWrite, and by TaskCreate and TaskUpdate one change at a time', async ($, on) => {
  const { posts, clock } = world(on)
  let made = 0
  on('tool.call', (_$, e) => ({ result: e.tool === 'TaskCreate' ? { task: { id: String(++made) } } : {}, text: 'ok' }))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()
  const before = posts.length

  await $.tool.call({ tool: 'TodoWrite', todos: [{ content: 'Read the intake', status: 'in_progress' }, { content: 'Test it', status: 'pending' }] })
  await $.tool.call({ tool: 'TodoWrite', todos: [{ content: 'Read the intake', status: 'in_progress' }, { content: 'Test it', status: 'pending' }] })
  await $.tool.call({ tool: 'TaskCreate', subject: 'Write the migration', description: 'x' })
  await $.tool.call({ tool: 'TaskUpdate', taskId: '1', status: 'completed' })
  await $.tool.call({ tool: 'TaskUpdate', taskId: '1', status: 'completed' })
  await clock.settle()

  const told = tasksTold(posts.slice(before))
  expect(told.length).toBe(3)
  expect(told[0]).toEqual([
    { id: '1', text: 'Read the intake', status: 'in_progress' },
    { id: '2', text: 'Test it', status: 'pending' },
  ])
  expect(told[1].at(-1)).toEqual({ id: '1', text: 'Write the migration', status: 'pending' })
  expect(told[2].at(-1)).toEqual({ id: '1', text: 'Write the migration', status: 'completed' })
})

test('every tenth call of the session\'s own agent tells Fleet ten more, and a subagent\'s count for nothing', async ($, on) => {
  const { posts, clock } = world(on)
  on('tool.call', () => ({ result: {}, text: 'ok' }))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()
  const before = posts.length

  for (let at = 0; at < 9; at++) await $.tool.call({ tool: 'Read', file_path: '/repos/armada/README.md' })
  for (let at = 0; at < 5; at++) await $.tool.call({ tool: 'Read', file_path: '/repos/armada/README.md', agentId: 'sub1' })
  await clock.settle()
  expect(facts(posts.slice(before)).filter(fact => fact.kind === 'calls')).toEqual([])

  await $.tool.call({ tool: 'Read', file_path: '/repos/armada/README.md' })
  await clock.settle()
  expect(facts(posts.slice(before)).filter(fact => fact.kind === 'calls')).toEqual([{ kind: 'calls', calls: 10 }])
})

const stored = (id: string, subject: string, status: string) => JSON.stringify({ id, subject, description: 'x', status, blocks: [], blockedBy: [] })

test('a subject rename is heard', async ($, on) => {
  const { posts, clock } = world(on)
  let made = 0
  on('tool.call', (_$, e) => ({ result: e.tool === 'TaskCreate' ? { task: { id: String(++made) } } : {}, text: 'ok' }))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()
  const before = posts.length

  await $.tool.call({ tool: 'TaskCreate', subject: 'Mock walk' })
  await $.tool.call({ tool: 'TaskUpdate', taskId: '1', subject: 'Mock walk: Queue for later' })
  await clock.settle()

  expect(tasksTold(posts.slice(before)).at(-1)).toEqual([{ id: '1', text: 'Mock walk: Queue for later', status: 'pending' }])
})

test('a TaskUpdate for a task this process never saw made still reaches Fleet, as the store and the change together', async ($, on) => {
  // The process was replaced after the list was made: Claude Code kept the list on disk, the mod's memory is empty.
  // What the disk holds here is the list as it stood before the update, so the change is applied on top of it.
  const { posts, clock } = world(on, {
    tasksOnDisk: {
      '1.json': stored('1', 'Plan', 'completed'),
      '2.json': stored('2', 'Mock walk', 'in_progress'),
      '3.json': stored('3', 'Wire it', 'pending'),
      '10.json': stored('10', 'Land it', 'pending'),
      '.lock': '',
    },
  })
  on('tool.call', () => ({ result: {}, text: 'ok' }))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()
  const before = posts.length

  await $.tool.call({ tool: 'TaskUpdate', taskId: '2', subject: 'Mock walk: renamed' })
  await $.tool.call({ tool: 'TaskUpdate', taskId: '2', status: 'completed' })
  await clock.settle()

  const told = tasksTold(posts.slice(before))
  expect(told[0]).toEqual([
    { id: '1', text: 'Plan', status: 'completed' },
    { id: '2', text: 'Mock walk: renamed', status: 'in_progress' },
    { id: '3', text: 'Wire it', status: 'pending' },
    { id: '10', text: 'Land it', status: 'pending' },
  ])
  expect(told.at(-1)?.map(one => one.status)).toEqual(['completed', 'completed', 'pending', 'pending'])
})

test('the store is read once, and a TaskCreate after it keeps the tasks already there', async ($, on) => {
  const { posts, clock, attempts } = world(on, { tasksOnDisk: { '1.json': stored('1', 'Plan', 'completed'), '2.json': stored('2', 'Build', 'pending') } })
  on('tool.call', (_$, e) => ({ result: e.tool === 'TaskCreate' ? { task: { id: '3' } } : {}, text: 'ok' }))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()
  const before = posts.length

  await $.tool.call({ tool: 'TaskCreate', subject: 'Ship', agentId: 'a1' })
  await clock.settle()

  expect(tasksTold(posts.slice(before)).at(-1)).toEqual([
    { id: '1', text: 'Plan', status: 'completed' },
    { id: '2', text: 'Build', status: 'pending' },
    { id: '3', text: 'Ship', status: 'pending', by: 'a1' },
  ])
  expect(attempts.reads).toBeGreaterThan(0)
})

test('a list named by CLAUDE_CODE_TASK_LIST_ID is read from its own directory', async ($, on) => {
  const { posts, clock, attempts } = world(on, { env: { CLAUDE_CODE_TASK_LIST_ID: 'team/one' }, tasksOnDisk: { '1.json': stored('1', 'Plan', 'pending') } })
  on('tool.call', () => ({ result: {}, text: 'ok' }))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()
  const before = posts.length

  await $.tool.call({ tool: 'TaskUpdate', taskId: '1', status: 'completed' })
  await clock.settle()

  expect(attempts.listed).toEqual(['/home/user/.claude/tasks/team-one'])
  expect(tasksTold(posts.slice(before))).toEqual([[{ id: '1', text: 'Plan', status: 'completed' }]])
})

test('a task named by neither the process nor the harness\'s store is not invented', async ($, on) => {
  const { posts, clock } = world(on)
  on('tool.call', () => ({ result: {}, text: 'ok' }))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()
  const before = posts.length

  await $.tool.call({ tool: 'TaskUpdate', taskId: '4', status: 'completed' })
  await clock.settle()

  expect(tasksTold(posts.slice(before))).toEqual([])
})

test('a subagent\'s TodoWrite is its own list and is not told, but its task calls write the session\'s', async ($, on) => {
  const { posts, clock } = world(on)
  let made = 0
  on('tool.call', (_$, e) => ({ result: e.tool === 'TaskCreate' ? { task: { id: String(++made) } } : {}, text: 'ok' }))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()
  const before = posts.length

  await $.tool.call({ tool: 'TodoWrite', agentId: 'a1', todos: [{ content: 'Map the claims', status: 'pending' }] })
  await clock.settle()
  expect(tasksTold(posts.slice(before))).toEqual([])

  await $.tool.call({ tool: 'TaskCreate', subject: 'parent-a' })
  await $.tool.call({ tool: 'TaskCreate', subject: 'child-b', agentId: 'a1' })
  await $.tool.call({ tool: 'TaskUpdate', taskId: '2', status: 'completed', agentId: 'a1' })
  await $.tool.call({ tool: 'TaskUpdate', taskId: '1', status: 'in_progress', agentId: 'a1' })
  await clock.settle()

  const told = tasksTold(posts.slice(before))
  expect(told[1]).toEqual([
    { id: '1', text: 'parent-a', status: 'pending' },
    { id: '2', text: 'child-b', status: 'pending', by: 'a1' },
  ])
  // Whoever updates a task, it stays its maker's.
  expect(told.at(-1)).toEqual([
    { id: '1', text: 'parent-a', status: 'in_progress' },
    { id: '2', text: 'child-b', status: 'completed', by: 'a1' },
  ])
})

test('leaving plan mode is told, and a plan turned down is not', async ($, on) => {
  const { posts, clock } = world(on)
  let accepted = true
  on('tool.call', () => (accepted ? { result: {}, text: 'ok' } : { result: {}, text: 'rejected', isError: true }))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()
  const before = posts.length

  await $.tool.call({ tool: 'ExitPlanMode', plan: 'Do the thing' })
  accepted = false
  await $.tool.call({ tool: 'ExitPlanMode', plan: 'Do the other thing' })
  await clock.settle()

  expect(facts(posts.slice(before)).filter(fact => fact.kind === 'plan_left')).toEqual([{ kind: 'plan_left' }])
})

test('a commit and a push are told from what git printed, and one that printed nothing to show is not', async ($, on) => {
  const { posts, clock } = world(on)
  let printed = ''
  on('tool.call', { tool: 'Bash' }, () => ({ result: { stdout: printed, stderr: '', interrupted: false }, text: printed }))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()
  const before = posts.length

  printed = '[armada/topic 1a2b3c4] Pin the clock'
  await $.tool.call({ tool: 'Bash', command: 'git commit -m "Pin the clock"' })
  printed = 'To https://github.com/Mele-Labs/armada.git\n * [new branch]      armada/topic -> armada/topic'
  await $.tool.call({ tool: 'Bash', command: 'git push -u origin armada/topic' })
  printed = 'Everything up-to-date'
  await $.tool.call({ tool: 'Bash', command: 'git push' })
  printed = 'nothing to commit, working tree clean'
  await $.tool.call({ tool: 'Bash', command: 'git commit -m again' })
  await clock.settle()

  const road = facts(posts.slice(before)).filter(fact => fact.kind === 'committed' || fact.kind === 'pushed')
  expect(road).toEqual([
    { kind: 'committed', branch: 'armada/topic', commit: '1a2b3c4', subject: 'Pin the clock' },
    { kind: 'pushed', branch: 'armada/topic' },
  ])
})

test('what a subagent edits, reads, runs and commits is told as the session\'s own work', async ($, on) => {
  const { posts, clock } = world(on)
  let printed = ''
  on('tool.call', { tool: 'Bash' }, () => ({ result: { stdout: printed, stderr: '', interrupted: false }, text: printed }))
  on('tool.call', { tool: 'Edit' }, () => ({ result: {}, text: 'ok' }))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()
  const before = posts.length

  await $.tool.call({ tool: 'Edit', file_path: '/repos/armada/src/a.rs', old_string: 'a', new_string: 'b', agentId: 'sub1' })
  printed = '[armada/topic 1a2b3c4] Pin the clock'
  await $.tool.call({ tool: 'Bash', command: 'git commit -m "Pin the clock"', agentId: 'sub1' })
  await clock.settle()

  const told = facts(posts.slice(before)).filter(fact => ['touched', 'ran', 'committed'].includes(fact.kind))
  expect(told.map(fact => fact.kind).sort()).toEqual(['committed', 'ran', 'touched'])
})

test('a permission prompt the terminal opens is told to Fleet, and the call ending settles it', async ($, on) => {
  const { posts, clock } = world(on)
  let answer: () => void = () => undefined
  const answered = new Promise<void>(resolve => (answer = resolve))
  on('tool.call', async () => (await answered, { result: {}, text: 'built' }))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()
  const before = posts.length

  // The engine opens its prompt inside the call, and the call ends once it is answered.
  const call = $.tool.call({ tool: 'Bash', command: 'cargo build\n--release' })
  await $.classic.PermissionRequest({ tool_name: 'Bash', tool_input: { command: 'cargo build\n--release' } })
  await clock.settle()
  expect(facts(posts.slice(before))).toEqual([{ kind: 'permission_asked', tool: 'Bash', detail: 'cargo build' }])

  answer()
  await call
  await clock.settle()
  expect(kinds(posts.slice(before)).filter(kind => kind.startsWith('permission'))).toEqual(['permission_asked', 'permission_settled'])
})

test('a permission a settings hook decided opens no prompt, and nothing is told', async ($, on) => {
  const { posts, clock } = world(on, { permission: { decision: { behavior: 'allow' } } })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.classic.PermissionRequest({ tool_name: 'Bash', tool_input: { command: 'cargo build' } })
  await clock.settle()

  expect(kinds(posts)).not.toContain('permission_asked')
})

test('a prompt still open when Fleet comes back is told again, and one the next turn closed is not', async ($, on) => {
  let up = true
  const { posts, clock } = world(on, { running: () => up })
  on('tool.call', () => ({ result: {}, text: 'ok' }))
  const outage = async (then: () => Promise<unknown>) => {
    up = false
    await then()
    await clock.settle()
    up = true
    await clock.advance(31_000)
  }
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()

  await outage(() => $.classic.PermissionRequest({ tool_name: 'Read', tool_input: { file_path: '/etc/hosts' } }))
  await $.tool.call({ tool: 'Read', file_path: '/repos/armada/a.rs' })
  await clock.settle()
  expect(facts(posts)).toContainEqual({ kind: 'permission_asked', tool: 'Read', detail: '/etc/hosts' })

  await $.prompt.submit({ text: 'go on', origin: { kind: 'user' } } as never)
  await clock.settle()
  const before = posts.length
  await outage(() => $.tool.call({ tool: 'Read', file_path: '/repos/armada/c.rs' }))
  await $.tool.call({ tool: 'Read', file_path: '/repos/armada/b.rs' })
  await clock.settle()
  expect(kinds(posts.slice(before))).not.toContain('permission_asked')
})

/**
 * The engine beneath the mod for a call the terminal asks permission for. The first run waits on the
 * terminal's prompt, which answers only where `terminal` says how; a run after it is the one Bridge
 * allowed, held until `release` where a test reads `tool.check` meanwhile.
 */
function prompting(on: Parameters<Parameters<typeof test>[1]>[1], terminal?: Promise<{ result: unknown; text: string }>, release?: Promise<void>) {
  const seen = { runs: 0 }
  on('tool.check', () => ({ decision: 'ask' as const }))
  on('tool.call', async () => {
    seen.runs += 1
    if (seen.runs === 1) return terminal ?? new Promise<never>(() => undefined)
    await release
    return { result: {}, text: 'built' }
  })
  return seen
}

const BASH = { tool_name: 'Bash', tool_input: { command: 'cargo build' } }

test('a permission allowed in Bridge runs the call again without the prompt, and the first run is dropped', async ($, on) => {
  const { questions, clock } = world(on, { polls: ['{"outcome":"answered","updated_input":{"command":"cargo build"}}'] })
  let release: () => void = () => undefined
  const seen = prompting(on, undefined, new Promise<void>(resolve => (release = resolve)))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()

  const call = $.tool.call({ tool: 'Bash', command: 'cargo build' })
  await $.classic.PermissionRequest(BASH)
  await clock.settle()
  expect(questions[0]).toEqual({ kind: 'asks', session_id: 'S1', tool: 'Bash', input: { command: 'cargo build' } })
  // While the allowed run goes, the check lets it past the prompt; once it is over, it asks again.
  expect((await $.tool.check({ tool: 'Bash', input: { command: 'cargo build' } })).decision).toBe('allow')
  release()
  const ran = await call

  expect(ran.text).toBe('built')
  expect(seen.runs).toBe(2)
  expect((await $.tool.check({ tool: 'Bash', input: { command: 'cargo build' } })).decision).toBe('ask')
})

test('a permission refused in Bridge is denied with what Fleet said', async ($, on) => {
  const { clock } = world(on, { polls: ['{"outcome":"refused","message":"The person refused this."}'] })
  prompting(on)
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()

  const call = $.tool.call({ tool: 'Bash', command: 'cargo build' })
  await $.classic.PermissionRequest(BASH)
  const ran = await call

  expect(ran.deny).toContain('The person refused this.')
})

test('a permission answered in the terminal first tells Fleet it was settled', async ($, on) => {
  const { questions, clock } = world(on)
  let answer: (ran: { result: unknown; text: string }) => void = () => undefined
  prompting(on, new Promise(resolve => (answer = resolve)))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()

  const call = $.tool.call({ tool: 'Bash', command: 'cargo build' })
  await $.classic.PermissionRequest(BASH)
  await clock.settle()
  answer({ result: {}, text: 'built in the terminal' })
  const ran = await call
  await clock.settle()

  expect(ran.text).toBe('built in the terminal')
  expect(questions).toContainEqual({ kind: 'settled', session_id: 'S1', answered: true })
})

const OWED = [
  '**BLUF:** Built and tested.',
  '',
  '## Actions',
  '',
  '| # | Action | Owner | Status |',
  '|---|---|---|---|',
  '| 1 | Opened PR #2415 | Agent | Done |',
  '| 2 | Approve PR #2415 | Nick | Not started |',
  '| 3 | Restart Fleet after it merges | Nick | Not started |',
].join('\n')

const stop = (reply: string, more: Record<string, unknown> = {}) =>
  ({ stop_hook_active: false, last_assistant_message: reply, ...more }) as never

const waitingPosts = (posts: Posted[]) => posts.filter(one => one.url.endsWith('/sessions/waiting'))

// What Fleet holds for the session: one item the agent set, one Fleet raised for a page it showed.
const HELD = {
  session: {
    id: 'S1',
    waiting_for: [
      { id: 'pick', text: 'Pick the slot pool', since: '2026-10-16T10:00:00Z', source: 'agent', act: { kind: 'answer', target: '' } },
      { id: 'walk:http://localhost:5173/', text: 'Walk the band', since: '2026-10-16T10:01:00Z', source: 'walk' },
    ],
  },
  rows: [],
}

test('a reply that names what the owner owes outside waiting_for is sent back once, then put in the list', async ($, on) => {
  const { posts, clock } = world(on, { thread: HELD })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.prompt.submit({ text: 'build the detector', origin: { kind: 'user' } } as never)
  await clock.settle()

  const first = await $.classic.Stop(stop(OWED))
  expect(first.block).toBe(
    [
      'Your reply names what the owner owes, and the armada waiting_for list does not hold it:',
      '- Approve PR #2415',
      '- Restart Fleet after it merges',
      'Put each in waiting_for with its own id, one short line and an act, passing session_id S1, then end the turn. Do not repeat them as a table.',
    ].join('\n'),
  )

  // The session ignored it and wrote the table again: the stop stands, and Fleet gets the lines.
  const second = await $.classic.Stop(stop(OWED, { stop_hook_active: true }))
  await clock.settle()
  expect(second.block).toBeUndefined()
  // The agent's own item stays; the one Fleet raised is Fleet's to keep.
  expect(waitingPosts(posts).map(one => one.body)).toEqual([
    {
      session_id: 'S1',
      items: [
        { id: 'pick', text: 'Pick the slot pool', act: { kind: 'answer', target: '' } },
        { id: 'reply:1', text: 'From the reply: Approve PR #2415' },
        { id: 'reply:2', text: 'From the reply: Restart Fleet after it merges' },
      ],
    },
  ])

  // Never again in the same turn.
  expect((await $.classic.Stop(stop(OWED, { stop_hook_active: true }))).block).toBeUndefined()
  await clock.settle()
  expect(waitingPosts(posts)).toHaveLength(1)
})

test('a session that puts the lines in waiting_for after it is sent back stops, and nothing is posted for it', async ($, on) => {
  const { posts, clock } = world(on)
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.prompt.submit({ text: 'build the detector', origin: { kind: 'user' } } as never)
  expect((await $.classic.Stop(stop(OWED))).block).toBeDefined()

  await $.tool.call({
    tool: 'mcp__armada-fleet__waiting_for',
    body: { session_id: 'S1', waiting_for: [{ id: 'merge', text: 'Approve the detector', act: { kind: 'approve_pr', target: '#2415' } }] },
  } as never)
  const second = await $.classic.Stop(stop('Both are in Bridge now.', { stop_hook_active: true }))
  await clock.settle()
  expect(second.block).toBeUndefined()
  expect(waitingPosts(posts)).toEqual([])
})

test('a list that already holds the lines, a question asked, or no lines at all lets the turn stop', async ($, on) => {
  const { clock } = world(on)
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.prompt.submit({ text: 'one', origin: { kind: 'user' } } as never)
  await $.tool.call({
    tool: 'mcp__armada-fleet__waiting_for',
    body: {
      session_id: 'S1',
      waiting_for: [
        { id: 'merge', text: 'Approve PR #2415' },
        { id: 'restart', text: 'Restart Fleet once merged', act: { kind: 'run', target: 'scripts/restart' } },
      ],
    },
  } as never)
  expect((await $.classic.Stop(stop(OWED))).block).toBeUndefined()

  await $.prompt.submit({ text: 'two', origin: { kind: 'user' } } as never)
  expect((await $.classic.Stop(stop('Built it. Nothing is owed.'))).block).toBeUndefined()
  await clock.settle()
})

test('each turn may be sent back once of its own', async ($, on) => {
  world(on)
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.prompt.submit({ text: 'one', origin: { kind: 'user' } } as never)
  expect((await $.classic.Stop(stop('QUESTION: Which slot pool?'))).block).toBeDefined()
  expect((await $.classic.Stop(stop('QUESTION: Which slot pool?', { stop_hook_active: true }))).block).toBeUndefined()
  await $.prompt.submit({ text: 'two', origin: { kind: 'user' } } as never)
  expect((await $.classic.Stop(stop('QUESTION: Which branch?'))).block).toBeDefined()
})

test('a Drone stopping is never sent back', async ($, on) => {
  world(on, { env: { ARMADA_DRONE: '1' } })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  expect((await $.classic.Stop(stop(OWED))).block).toBeUndefined()
})

test('a subagent stopping is never sent back', async ($, on) => {
  const { posts, clock } = world(on)
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.prompt.submit({ text: 'one', origin: { kind: 'user' } } as never)
  expect((await $.classic.Stop(stop(OWED, { agent_id: 'a1b2', agent_type: 'Explore' }))).block).toBeUndefined()
  await clock.settle()
  expect(waitingPosts(posts)).toEqual([])
})

test('a stop another hook already blocked keeps its reason', async ($, on) => {
  const { posts, clock } = world(on, { stopBlock: 'Finish the tests first.' })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.prompt.submit({ text: 'one', origin: { kind: 'user' } } as never)
  expect((await $.classic.Stop(stop(OWED))).block).toBe('Finish the tests first.')
  await clock.settle()
  expect(waitingPosts(posts)).toEqual([])
})

test('a list Fleet cannot be read for is posted only from a waiting_for call this load saw, and never from nothing', async ($, on) => {
  const { posts, clock } = world(on)
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.prompt.submit({ text: 'one', origin: { kind: 'user' } } as never)
  expect((await $.classic.Stop(stop(OWED))).block).toBeDefined()
  await $.classic.Stop(stop(OWED, { stop_hook_active: true }))
  await clock.settle()
  // The agent may have set a list before this load: posting over it would drop its items.
  expect(waitingPosts(posts)).toEqual([])

  await $.prompt.submit({ text: 'two', origin: { kind: 'user' } } as never)
  await $.tool.call({
    tool: 'mcp__armada-fleet__waiting_for',
    body: { session_id: 'S1', waiting_for: [{ id: 'pick', text: 'Pick the slot pool' }] },
  } as never)
  expect((await $.classic.Stop(stop(OWED))).block).toBeDefined()
  await $.classic.Stop(stop(OWED, { stop_hook_active: true }))
  await clock.settle()
  expect(waitingPosts(posts).map(one => one.body.items.map((item: { id: string }) => item.id))).toEqual([
    ['pick', 'reply:1', 'reply:2'],
  ])
})

// What the window lists, as the engine's summary breakdown carries it: a skill, a subagent, an MCP
// server's tools, and a memory file from each place Armada names.
const LISTED = {
  categories: [],
  totalTokens: 1,
  maxTokens: 200_000,
  rawMaxTokens: 200_000,
  gridRows: [],
  memoryFiles: [
    { path: '/home/user/.claude/CLAUDE.md', type: 'User', tokens: 40 },
    { path: '/repos/armada/CLAUDE.md', type: 'Project', tokens: 800 },
    { path: '/etc/claude-code/CLAUDE.md', type: 'Managed', tokens: 9 },
  ],
  mcpTools: [
    { name: 'mcp__github__get_me', serverName: 'github', tokens: 30, isLoaded: false },
    { name: 'mcp__github__get_pr', serverName: 'github', tokens: 31, isLoaded: true },
  ],
  agents: [
    { agentType: 'reviewer', source: 'projectSettings', tokens: 12 },
    { agentType: 'productivity:planner', source: 'plugin', tokens: 7 },
  ],
  skills: {
    totalSkills: 4,
    includedSkills: 4,
    tokens: 40,
    skillFrontmatter: [
      { name: 'land', source: 'projectSettings', tokens: 10 },
      { name: 'notes', source: 'userSettings', tokens: 10 },
      { name: 'armada:walk', source: 'plugin', pluginName: 'armada', tokens: 10 },
      { name: 'productivity:start', source: 'plugin', pluginName: 'productivity', tokens: 10 },
    ],
  },
  isAutoCompactEnabled: true,
  apiUsage: null,
}

// A system prompt composed with Armada's own output style.
const ARMADA_STYLE = { model: 'claude-haiku-4-5', promptModel: 'claude-haiku-4-5', surfaces: [], tools: [], outputStyle: { name: 'Armada', isKeepingCodingInstructions: true }, traits: [] }

const equippedItems = (posts: Posted[]) => facts(posts).filter(fact => fact.kind === 'equipped').map(fact => fact.loadout.items)
const uses = (posts: Posted[]) => facts(posts).filter(fact => fact.kind === 'used').map(fact => fact.use)

test('the Loadout is told at the start as names and where each came from, never a body', async ($, on) => {
  const { posts, clock } = world(on, { breakdown: LISTED, env: { ARMADA_SESSION: 'S1' } })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()

  expect(equippedItems(posts)).toEqual([
    [
      { kind: 'skill', name: 'land', source: 'repository', tokens: 10 },
      { kind: 'skill', name: 'notes', source: 'kit', tokens: 10 },
      { kind: 'skill', name: 'armada:walk', source: 'armada', tokens: 10 },
      { kind: 'skill', name: 'productivity:start', source: 'plugin', from: 'productivity', tokens: 10 },
      { kind: 'subagent', name: 'reviewer', source: 'repository', tokens: 12 },
      { kind: 'subagent', name: 'productivity:planner', source: 'plugin', from: 'productivity', tokens: 7 },
      { kind: 'mcp_server', name: 'github', source: 'mcp', from: 'github', state: 'connected' },
      { kind: 'tool', name: 'mcp__github__get_me', source: 'mcp', from: 'github', state: 'deferred', tokens: 30 },
      { kind: 'tool', name: 'mcp__github__get_pr', source: 'mcp', from: 'github', state: 'loaded', tokens: 31 },
      { kind: 'memory_file', name: '~/.claude/CLAUDE.md', source: 'kit', tokens: 40 },
      { kind: 'memory_file', name: 'CLAUDE.md', source: 'repository', tokens: 800 },
      { kind: 'memory_file', name: '/etc/claude-code/CLAUDE.md', source: 'managed', tokens: 9 },
      { kind: 'rules', name: 'Armada session rules', source: 'armada' },
      { kind: 'injection', name: 'armadaSession', source: 'armada' },
    ],
  ])
})

test('the Loadout is told again only when it moved', async ($, on) => {
  const { posts, clock } = world(on, { breakdown: LISTED })
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()
  await $.session.measure({ context: { tokens: 1000, window: 200_000 }, rateLimits: [], changed: ['context'] })
  await clock.settle()
  expect(equippedItems(posts)).toHaveLength(1)

  await $.prompt.compose(ARMADA_STYLE)
  await $.prompt.compose(ARMADA_STYLE)
  await clock.settle()

  const told = equippedItems(posts)
  expect(told).toHaveLength(2)
  expect(told[1]).toContainEqual({ kind: 'output_style', name: 'Armada', source: 'armada' })
  expect(told[1]).not.toContainEqual(expect.objectContaining({ kind: 'rules' }))
})

test('a skill, an MCP tool, a ToolSearch and a subagent are told as uses under the turn they were in', async ($, on) => {
  const { posts, clock } = world(on, { breakdown: LISTED })
  on('tool.call', { tool: 'Skill' }, () => ({ result: { success: true, commandName: 'land' }, text: '' }))
  on('tool.call', { tool: 'mcp__github__get_me' }, () => ({ result: {}, text: 'a secret answer' }))
  on('tool.call', { tool: 'ToolSearch' }, () => ({
    result: { matches: ['mcp__github__get_me', 'WebFetch'], query: 'github', total_deferred_tools: 3, failed_mcp_servers: [{ name: 'linear', error: 'a secret error' }] },
    text: '',
  }))
  on('agent.spawn', () => ({ model: 'claude-haiku-4-5', agentId: 'A9' }))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await clock.settle()
  await clock.advance(5000)
  await $.prompt.submit({ text: 'land it', origin: { kind: 'user' } } as never)
  await clock.settle()
  // The prompt came five seconds after the mock clock's start.
  const turn = new Date(1_005_000).toISOString()

  await $.tool.call({ tool: 'Skill', skill: 'land' })
  await $.tool.call({ tool: 'mcp__github__get_me', agentId: 'A2' } as never)
  await $.tool.call({ tool: 'ToolSearch', query: 'github' } as never)
  await $.agent.spawn({ prompt: 'a secret task', description: 'review', subagentType: 'reviewer', model: 'haiku' } as never)
  await $.tool.call({ tool: 'Bash', command: 'ls' } as never).catch(() => undefined)
  await clock.settle()

  expect(uses(posts)).toEqual([
    { turn_started_at: turn, kind: 'skill', name: 'land', source: 'repository' },
    { turn_started_at: turn, kind: 'tool', name: 'mcp__github__get_me', source: 'mcp', from: 'github', by: 'A2' },
    { turn_started_at: turn, kind: 'tool', name: 'mcp__github__get_me', source: 'mcp', from: 'github' },
    { turn_started_at: turn, kind: 'tool', name: 'WebFetch', source: 'built_in' },
    { turn_started_at: turn, kind: 'subagent', name: 'reviewer', source: 'repository', model: 'haiku' },
  ])
  const last = equippedItems(posts).at(-1)
  expect(last).toContainEqual({ kind: 'mcp_server', name: 'linear', source: 'mcp', from: 'linear', state: 'failed' })
  expect(last).toContainEqual({ kind: 'tool', name: 'mcp__github__get_me', source: 'mcp', from: 'github', state: 'loaded', tokens: 30 })
  expect(last).toContainEqual({ kind: 'tool', name: 'WebFetch', source: 'built_in', state: 'loaded' })
  expect(JSON.stringify(posts)).not.toContain('secret')
})

test('a Drone tells no Loadout and no use', async ($, on) => {
  const { posts, clock } = world(on, { breakdown: LISTED, env: { ARMADA_DRONE: '1' } })
  on('tool.call', { tool: 'Skill' }, () => ({ result: { success: true, commandName: 'land' }, text: '' }))
  await $.session.start({ cwd: '/repos/armada', surface: null, isInteractive: false })
  await $.tool.call({ tool: 'Skill', skill: 'land' })
  await $.prompt.compose(ARMADA_STYLE)
  await clock.settle()

  expect(posts).toEqual([])
})
