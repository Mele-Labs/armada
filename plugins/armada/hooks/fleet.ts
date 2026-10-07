// What Fleet's intake takes: `crates/ipc/src/sessions.rs`, `SessionReport`. Only
// shapes here; the code that sends them is in `register.ts`, since the engine
// follows `$` into one file only.

import type { Engine } from 'claude-code'

export type Door = Pick<Engine, 'clock' | 'env' | 'fs' | 'http'>

export type Report = {
  harness: string
  session_id: string
  fact: Fact
}

export type Usage = {
  context_tokens?: number
  context_window?: number
  cost_micros?: number
}

export type Fact =
  | { kind: 'started'; cwd: string; title?: string; origin: 'terminal' }
  | { kind: 'titled'; title: string; named?: boolean }
  | { kind: 'moved'; cwd: string }
  | {
      kind: 'attached'
      attachment: { kind: string; target: string; detail?: Record<string, string> }
    }
  | {
      kind: 'settled'
      attachment: { kind: string; target: string }
      state: 'spent' | 'given_back'
    }
  | { kind: 'measured'; usage: Usage }
  | {
      kind: 'tuned'
      model?: string
      effort?: string
      mode?: 'ask' | 'auto' | 'accept_edits' | 'plan'
      commands?: { name: string; says: string }[]
    }
  | { kind: 'turn_completed' }
  | { kind: 'ended'; reason: string }
