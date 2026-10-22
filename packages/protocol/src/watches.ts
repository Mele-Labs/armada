// Watches: a command Fleet runs on a cadence for one Job, and the agent's weighing of a failed reading.
// Hand-mirrored from `crates/ipc/src/watches.rs`. `docs/practices/protocol.md`, *Watches on the wire*.
//
// The header rules in `protocol.ts` hold here. The line a watch's command prints is Fleet's to read and is not
// mirrored: Bridge sees a reading only as `WatchReading`.

/** What a reading, or one part of it, says. */
export type WatchReadingState = "waiting" | "running" | "passed" | "failed";

/** Where a watch stands. `gave_up`, `broken` and `stopped` are ends no reading says. */
export type WatchState = WatchReadingState | "gave_up" | "broken" | "stopped";

export type WatchPart = { name: string; state: WatchReadingState };

/** A part the agent weighed as not its change's doing, and why. Drawn struck through. */
export type WatchSetAside = { part: string; sentence: string };

/** One reading: what the command's line said, or why it gave none. */
export type WatchReading =
  | { kind: "read"; at: string; state: WatchReadingState; finished: boolean; says: string; link?: string; parts?: WatchPart[] }
  | { kind: "broken"; at: string; why: string; output?: string };

export type WatchSummary = {
  /** What `weigh_watch` and `stop_watch` name. */
  id: string;
  session_id: string;
  name: string;
  /** Whole, every value filled: what Fleet runs for the Job. */
  command: string[];
  /** Absent where the agent gave its own command. */
  recipe?: string;
  every_seconds: number;
  gives_up_after_seconds: number;
  /** A pull request number. */
  follows?: number;
  state: WatchState;
  /** False once it has ended, whichever way. */
  live: boolean;
  started_at: string;
  last?: WatchReading;
  set_aside?: WatchSetAside[];
};

/** `list_watches`: a Job's watches, live and ended, the newest first. */
export type WatchList = { watches: WatchSummary[] };

/** `start_watch`: a recipe with its values, or the agent's own command with `every`. */
export type StartWatch = {
  session_id?: string;
  name: string;
  follows?: number;
  set_aside?: WatchSetAside[];
} & (
  | { recipe: string; values?: Record<string, string> }
  | { command: string[]; every: string; gives_up_after?: string }
);

/** `weigh_watch`. `ours` ends the watch failed; `not_ours` sets its parts aside. */
export type WeighWatch = { session_id?: string; watch: string } & (
  | { verdict: "ours" }
  | { verdict: "not_ours"; sentence: string; parts: string[] }
);

/** `stop_watch`: the person's Stop, or the agent's. */
export type StopWatch = { session_id?: string; watch: string };
