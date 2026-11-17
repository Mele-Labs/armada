import { expect, test } from 'claude-code/testing'

import { placeOfCall, staticPrefix } from './handoff'

const ROOT = '/repos/armada'

test('a glob is read as the folder in front of its first wildcard', () => {
  expect(staticPrefix('crates/fleet/**/*.rs')).toBe('crates/fleet')
  expect(staticPrefix('**/*.rs')).toBe('')
  expect(staticPrefix('*.ts')).toBe('')
  expect(staticPrefix('crates/{fleet,api}/src')).toBe('crates')
  expect(staticPrefix('crates/fleet/src/lib.rs')).toBe('crates/fleet/src/lib.rs')
})

test('a Read reaches its file, as a path from the root', () => {
  expect(placeOfCall('Read', { file_path: `${ROOT}/crates/fleet/src/lib.rs` }, ROOT)).toBe('crates/fleet/src/lib.rs')
  expect(placeOfCall('Read', { file_path: 'crates/fleet/src/lib.rs' }, ROOT)).toBe('crates/fleet/src/lib.rs')
  expect(placeOfCall('Read', { file_path: './crates/fleet/src/lib.rs' }, ROOT)).toBe('crates/fleet/src/lib.rs')
})

test('a file outside the root, or the root itself, reaches no feature', () => {
  expect(placeOfCall('Read', { file_path: '/etc/hosts' }, ROOT)).toBeUndefined()
  expect(placeOfCall('Read', { file_path: '/repos/armada-other/a.rs' }, ROOT)).toBeUndefined()
  expect(placeOfCall('Grep', { path: ROOT }, ROOT)).toBeUndefined()
  expect(placeOfCall('Grep', { path: '.' }, ROOT)).toBeUndefined()
  expect(placeOfCall('Read', { file_path: '../elsewhere/a.rs' }, ROOT)).toBeUndefined()
})

test('a Grep or Glob reaches the folder it searches, narrowed by the folder in front of its glob', () => {
  expect(placeOfCall('Grep', { path: `${ROOT}/crates/fleet` }, ROOT)).toBe('crates/fleet')
  expect(placeOfCall('Grep', { path: 'crates', glob: 'fleet/**/*.rs' }, ROOT)).toBe('crates/fleet')
  expect(placeOfCall('Grep', { glob: '*.rs' }, ROOT)).toBeUndefined()
  expect(placeOfCall('Glob', { pattern: 'crates/fleet/**/*.rs' }, ROOT)).toBe('crates/fleet')
  expect(placeOfCall('Glob', { pattern: '**/*.rs', path: `${ROOT}/crates/api` }, ROOT)).toBe('crates/api')
  expect(placeOfCall('Glob', { pattern: `${ROOT}/crates/api/**/*.rs` }, ROOT)).toBe('crates/api')
  expect(placeOfCall('Bash', { path: 'crates' }, ROOT)).toBeUndefined()
})
