// What Fleet's intake takes: `crates/ipc/src/sessions.rs`, `SessionReport`. Only
// shapes here; the code that sends them is in `register.ts`, since the engine
// follows `$` into one file only.

import type { Engine } from 'claude-code'

import type { ContextBreakdown, LoadoutItem, LoadoutUse, PlanWindow } from './facts'

export type Door = Pick<Engine, 'clock' | 'env' | 'fs' | 'http'>

export type Report = {
  harness: string
  session_id: string
  fact: Fact
}

/** What Fleet answers a terminal question with (`TerminalAsked`, `crates/ipc/src/hosted_sessions.rs`). */
export type Asked =
  | { outcome: 'asked'; call: string }
  | { outcome: 'waiting' }
  | { outcome: 'answered'; updated_input: unknown }
  | { outcome: 'refused'; message: string }
  | { outcome: 'gone' }

export type Usage = {
  context_tokens?: number
  context_window?: number
  cost_micros?: number
}

/** One task of the agent's list. `id` is its task tool's, or its place in a `TodoWrite` list. */
/** `by` is the subagent that made the task, absent for the session's own. */
export type Task = { id: string; text: string; status: 'pending' | 'in_progress' | 'completed'; by?: string }

export type Fact =
  | { kind: 'started'; cwd: string; title?: string; origin: 'terminal'; mod_version?: string }
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
  | { kind: 'measured'; usage: Usage; plan?: PlanWindow[]; context_breakdown?: ContextBreakdown }
  | {
      kind: 'tuned'
      model?: string
      effort?: string
      mode?: 'ask' | 'auto' | 'accept_edits' | 'plan'
      commands?: { name: string; says: string }[]
      mod_version?: string
    }
  | { kind: 'turn_started' }
  | { kind: 'turn_completed'; said?: string }
  | { kind: 'limited'; resets_at: string }
  | { kind: 'signed_out' }
  | { kind: 'touched'; paths: string[]; act?: 'write' | 'edit'; by?: string }
  | { kind: 'read'; path: string }
  | { kind: 'ran'; command: string }
  | { kind: 'wrote' }
  | { kind: 'tasks'; tasks: Task[] }
  | { kind: 'plan_left' }
  | { kind: 'committed'; branch?: string }
  | { kind: 'pushed'; branch?: string }
  | { kind: 'permission_asked'; tool: string; detail?: string }
  | { kind: 'permission_settled' }
  | { kind: 'ended'; reason: string }
  // The Loadout: the whole list, replacing the last, and one use in the turn it began in.
  | { kind: 'equipped'; loadout: { items: LoadoutItem[] } }
  | { kind: 'used'; use: LoadoutUse }
