// A Job's Commander, as TypeScript sees it. `crates/ipc/src/commander.rs`.
//
// **`helm.ts`'s shape, one subject over.** A Commander is a conversation on Helm's host keyed by the
// Job, so its socket opens, carries the thread so far and then what follows, and its rows are
// `turn.ts`'s `Saw`. The words and a lost session are Helm's own types. What this socket has that
// Helm's does not is `woke`: Fleet started a turn, and the fact that did. Nothing on it advances a
// Job: a Commander claims nothing, and the Checks and the Judge decide.

import type { Missed } from "./events";
import type { Saw, Voice } from "./turn";
import type { ProtocolId } from "./connection";
import type { HelmAsked, HelmFresh, HelmUnanswered } from "./helm";

/** `POST /jobs/:job_id/commander/ask` — what the owner says to a Job's Commander. Blank is refused before this is sent. */
export type AskCommander = {
  text: string;
};

/** What `ask_commander` answers with. **Not the reply**, which arrives on the socket. */
export type CommanderConversation = {
  job_id: string;
  /** A turn is being written, or the words are waiting for the next one. */
  replying: boolean;
  /** The next turn resumes a stored session. `false` starts a new one. */
  resumes: boolean;
};

/** One message on `GET /jobs/:job_id/commander/observe`'s socket. */
export type CommanderMessage =
  | ({ message: "opened" } & CommanderOpened)
  /** Fleet started a turn, and the fact that did. */
  | ({ message: "woke" } & CommanderWoke)
  /** What the owner said, as Fleet took it — on the thread when taken, not when its turn starts. */
  | ({ message: "asked" } & HelmAsked)
  /** `Shown` on the Rust side. `turn.ts`'s own row, tagged the same way. */
  | ({ message: "row"; ts: string; step?: string; by?: Voice } & Saw)
  | ({ message: "fresh" } & HelmFresh)
  | ({ message: "unanswered" } & HelmUnanswered)
  | ({ message: "missed" } & Missed);

/** The first message on every connection. */
export type CommanderOpened = {
  protocol_id: ProtocolId;
  job_id: string;
  /** Whether a turn was being written, or one was waiting, when this opened. */
  replying: boolean;
  /** Older messages the thread left out, because the backfill is bounded. */
  skipped: number;
};

/** Several wakes that arrived while a turn ran are one turn, and this names each reason once. */
export type CommanderWoke = {
  ts: string;
  why: string;
};

/** Whether a Job's Commander is at work. */
export type CommanderState = "idle" | "working";

/** `commander.changed`: a Job's Commander started or finished a turn. */
export type CommanderChanged = {
  job_id: string;
  state: CommanderState;
};
