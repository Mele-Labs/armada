// A Job reads and commands the Jobs it started, mirrored by hand from `crates/ipc/src/started_jobs.rs` and
// `session_summary.rs`. `docs/concepts/session/a-job-that-started-jobs.md`.
//
// Neither call names the Job it is for: Fleet places the caller by its connection. Bridge draws none of
// this; the types are here so the wire has one spelling on both sides.

import type { SessionOrigin, SessionState, SessionStanding, WaitingItem } from "./sessions";
import type { SessionWait, StartBase, StartCondition } from "./session-waits";
import type { WireError } from "./protocol";

/** Whether a hosted Job's turn is running. */
export type TurnState = "idle" | "working";

/** A pull request a Job holds that is still open. */
export type HeldPullRequest = {
  number: number;
  /** Empty where the forge has not been read for it yet. */
  title: string;
  /** `open` or `draft`; absent where the forge has not been read. */
  state?: string;
};

/** One Job in a line, as `list_sessions` gives it with `view=summary`. */
export type SessionSummary = {
  id: string;
  title?: string;
  manifest_id?: string;
  origin: SessionOrigin;
  state: SessionState;
  turn?: TurnState;
  last_turn_at?: string;
  last_seen_at: string;
  slot?: string;
  branch?: string;
  pull_requests?: HeldPullRequest[];
  mod_version?: string;
  tools_missing?: Array<"mod" | "door" | "task_tools">;
};

/** The workflow step a child is on, and what that step waits for. */
export type StartedStep = {
  index: number;
  label: string;
  /** One sentence for each end that has not come: `Waiting on PR #812: Approved`. */
  waiting_on?: string[];
};

/** How a child ended, and what it said it achieved with `all_done`. */
export type StartedResult = {
  /** `closed` for an end by Fleet or a person. */
  end_reason?: string;
  summary?: string;
};

/** One Job below the caller: its summary with these beside it. */
export type StartedJob = SessionSummary & {
  /** The Job that started it: the caller for a child, the child for a grandchild. */
  started_by: string;
  standing?: SessionStanding;
  /** It waits on its start card for the person to press Start. */
  held: boolean;
  /** Paused: it holds no slot and runs no turn. */
  paused: boolean;
  step?: StartedStep;
  /** What it waits on the person for, Fleet's own items (`start:<id>`, `unsent:<id>`) among them. */
  waiting_for?: WaitingItem[];
  /** The Job it waits on before it starts. Absent once it has started. */
  waits?: SessionWait;
  /** Present once it ended. */
  result?: StartedResult;
};

/** `list_started_sessions`: every Job below the caller, each child followed by its own descendants, oldest first at each level. */
export type StartedSessions = {
  jobs: StartedJob[];
};

/** What a Job does to a child. */
export type StartedAct = "withdraw" | "end" | "pause" | "resume" | "change" | "drop";

/** One entry of `command_started_sessions`: an act on one child. */
export type StartedCommand = {
  session_id: string;
  act: StartedAct;
  /** For `end`, which needs it: the child is told it. */
  why?: string;
  /** For `change`: when the child starts. */
  condition?: StartCondition;
  /** For `change`: where the child starts from. */
  base?: StartBase;
};

/** `command_started_sessions`' body. */
export type StartedCommands = {
  commands: StartedCommand[];
};

/** One entry of the answer: `done` with the child as it now stands, or `refused`. Exactly one is present. */
export type StartedAnswer = {
  done?: StartedJob;
  refused?: WireError;
};

/** The answer to `command_started_sessions`: one entry per command, in the order given. */
export type StartedAnswers = {
  commands: StartedAnswer[];
};
