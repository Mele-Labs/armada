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

/** Who put a card out: the Commander through `ask_the_owner`, or Fleet where the loop bound stopped it. */
export type CardBy = "commander" | "fleet";

/** One option of a card. `id` is what an answer names: its one-based place. */
export type CardOption = {
  id: string;
  label: string;
};

/**
 * The card a Job's Commander has out to the owner, which outlives the turn that raised it. #2166.
 * A Drone's question one subject over: asked once, a closed set to pick from, and it waits however
 * long he takes. **While it is out `JobSummary.asking` is true**, which puts the Job's call in the Cockpit's tray.
 */
export type CommanderCard = {
  card_id: string;
  job_id: string;
  by: CardBy;
  raised_at: string;
  /** What is wrong. */
  text: string;
  /** What is out of the Commander's reach. */
  reach: string;
  question: string;
  /** Two to four, in the order asked. */
  options: CardOption[];
};

/** `POST /jobs/:job_id/commander/card`. Sent by a Commander only. Every line has something in it; two to four distinct labels. */
export type AskTheOwner = {
  text: string;
  reach: string;
  question: string;
  options: string[];
};

/** `POST /jobs/:job_id/commander/card/answer` — his pick. No prose. */
export type AnswerCommanderCard = {
  card_id: string;
  option_id: string;
};

/** What `answer_commander_card` answers with. */
export type CommanderCardAnswered = {
  job_id: string;
  card_id: string;
  option_id: string;
  /** The Commander was woken with the answer. `false` where it could not start. */
  woke: boolean;
};

/** `commander.card`: a card went out, or the one out was answered (no `card`). `job.asking`'s shape. */
export type CommanderCardChanged = {
  job_id: string;
  card?: CommanderCard;
};

/** What a Job's Commander found broken in the Job's worktree or environment. #2170. */
export type Breakage = "merge_conflict" | "dependencies" | "missing_binary" | "worktree";

/**
 * `POST /jobs/:job_id/commander/repair` — send a repair Drone onto the Job's own branch and
 * worktree. A Commander's alone: the route refuses a call the door did not place as one. #2170.
 */
export type SendRepairDrone = {
  broke: Breakage;
};

/** What `send_repair_drone` answers with (202). The Drone has not finished. */
export type RepairSent = {
  job_id: string;
  /** The step the breakage stopped, restarted once the repair hands in. */
  step_id: string;
  drone_id: string;
  broke: Breakage;
};
