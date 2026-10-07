import { expect, test } from 'claude-code/testing'

import {
  effortOf,
  ghAct,
  modeOf,
  modelName,
  isDispatch,
  jobIdsIn,
  mayMoveBranch,
  needAct,
  micros,
  pullRequestsIn,
  senderOf,
  titleOf,
} from './facts'

test('a title is the first line of a prompt, cut, and never a command', () => {
  expect(titleOf('\n  fix the   ledger intake\nand its tests')).toBe('fix the ledger intake')
  expect(titleOf('/compact')).toBeUndefined()
  expect(titleOf('   \n  ')).toBeUndefined()
  expect(titleOf('x'.repeat(200))?.length).toBe(80)
})

test('a git command that may move the branch is told from one that cannot', () => {
  expect(mayMoveBranch('git checkout -b fleet/a')).toBe(true)
  expect(mayMoveBranch('cd x && git switch main')).toBe(true)
  expect(mayMoveBranch('git -C /tmp/x worktree add y')).toBe(true)
  expect(mayMoveBranch('git status')).toBe(false)
  expect(mayMoveBranch('git log --oneline')).toBe(false)
})

test('a pull request is read off its address, once', () => {
  const text = 'https://github.com/Mele-Labs/armada/pull/1853\nsee also https://github.com/Mele-Labs/armada/pull/1853 and https://github.com/o/r/pull/7'
  expect(pullRequestsIn(text).map(one => one.number)).toEqual(['1853', '7'])
  expect(pullRequestsIn('no address here')).toEqual([])
})

test('a gh command is read for the act and the number it names', () => {
  expect(ghAct('gh pr create --base main')).toEqual({ act: 'create', number: undefined })
  expect(ghAct('gh pr merge 12 --merge')).toEqual({ act: 'merge', number: '12' })
  expect(ghAct('gh pr merge #12')).toEqual({ act: 'merge', number: '12' })
  expect(ghAct('gh pr close')).toEqual({ act: 'close', number: undefined })
  expect(ghAct('gh pr view 12')).toBeUndefined()
})

test('a dispatch is an armada-fleet tool that makes or sends a Job', () => {
  expect(isDispatch('mcp__armada-fleet__propose_job')).toBe(true)
  expect(isDispatch('mcp__armada-fleet__list_jobs')).toBe(false)
  expect(isDispatch('mcp__other__propose_job')).toBe(false)
})

test('Job ids are ULIDs in an answer, each once and at most ten', () => {
  const one = '01J8ZQ4W8N5T6V7Y9B2C3D4E5F'
  expect(jobIdsIn(`created ${one} and ${one}`)).toEqual([one])
  expect(jobIdsIn('nothing')).toEqual([])
  const many = Array.from({ length: 12 }, (_, n) => `01J8ZQ4W8N5T6V7Y9B2C3D4E${String(10 + n)}`)
  expect(jobIdsIn(many.join(' ')).length).toBeLessThanOrEqual(10)
})

test('a sender is the teammate where there is one and the kind where there is not', () => {
  expect(senderOf({ kind: 'peer', teammate: 'researcher' })).toBe('researcher')
  expect(senderOf({ kind: 'peer' })).toBe('peer')
})

test('cost goes out in millionths of a dollar', () => {
  expect(micros(1.25)).toBe(1_250_000)
  expect(micros(undefined)).toBeUndefined()
})

test('an armada need is read in its three forms and in no other', () => {
  expect(needAct('armada need ./a.toml "a minor"')).toEqual({ act: 'declare', path: 'a.toml', what: 'a minor' })
  expect(needAct("cd x && armada need a.toml 'two words'")).toEqual({
    act: 'declare',
    path: 'a.toml',
    what: 'two words',
  })
  expect(needAct('armada need --took a.toml "23.5"')).toEqual({ act: 'took', path: 'a.toml', value: '23.5' })
  expect(needAct('armada need --release a.toml')).toEqual({ act: 'release', path: 'a.toml' })
  expect(needAct('armada need --status')).toBeUndefined()
  expect(needAct('armada need a.toml')).toBeUndefined()
  expect(needAct('echo armada need a.toml "x"')).toBeUndefined()
  expect(needAct('armada land')).toBeUndefined()
})

test('a model is named by its short name, a mode in Armada words and an effort whichever way it came', () => {
  expect(modelName('claude-sonnet-5-5')).toBe('sonnet')
  expect(modelName('some-other-model')).toBe('some-other-model')
  expect(modeOf('acceptEdits')).toBe('accept_edits')
  expect(modeOf('default')).toBe('ask')
  expect(modeOf('bypassPermissions')).toBeUndefined()
  expect(effortOf('low')).toBe('low')
  expect(effortOf({ level: 'high' })).toBe('high')
  expect(effortOf(undefined)).toBeUndefined()
})
