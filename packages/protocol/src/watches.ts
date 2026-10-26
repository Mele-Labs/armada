// Watches: a command Fleet runs on a cadence for one Job, and the agent's weighing of a failed reading.
// Hand-mirrored from `crates/ipc/src/watches.rs`. `docs/practices/protocol.md`, *Watches on the wire*.
//
// The header rules in `protocol.ts` hold here. The line a watch's command prints is Fleet's to read and is not
// mirrored: Bridge sees a reading only as `WatchReading`.

/** What a reading, or one part of it, says. */
export type WatchReadingState = "waiting" | "running" | "passed" | "failed";

/**
 * Where a watch stands. `not_ours` is every counted part passed while a part set aside still fails: never green, and an end once
 * the run is finished. `gave_up`, `broken` and `stopped` are ends no reading says.
 */
export type WatchState = WatchReadingState | "not_ours" | "gave_up" | "broken" | "stopped";

export type WatchPart = { name: string; state: WatchReadingState };

/** A part the agent weighed as not its change's doing, and why. Drawn struck through. */
export type WatchSetAside = { part: string; sentence: string };

/**
 * A run of readings that read the same, as `readings` carries one: `count` is how many in a row, absent for one, `at`
 * the first and `last_at` the last. `last` never carries a run.
 */
type WatchRun = { count?: number; last_at?: string };

/** One reading: what the command's line said, or why it gave none. A broken run's `output` is its last one's. */
export type WatchReading =
  | ({ kind: "read"; at: string; state: WatchReadingState; finished: boolean; says: string; link?: string; parts?: WatchPart[] } & WatchRun)
  | ({ kind: "broken"; at: string; why: string; output?: string } & WatchRun);

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
  /** The Requests environment whose variables the command is given. A name, never a value. */
  environment?: string;
  state: WatchState;
  /** False once it has ended, whichever way. */
  live: boolean;
  started_at: string;
  /** The newest reading as taken, its own `at`, never a run. */
  last?: WatchReading;
  set_aside?: WatchSetAside[];
  /** The kept readings, the newest 50, oldest first, `last` among them, back-to-back ones that read the same as one run. Absent is none. */
  readings?: WatchReading[];
  /** The agent's weighings, the oldest first. Absent is none. */
  verdicts?: WatchVerdict[];
  /** What a terminal watch waits for in its Job's terminal. Its `command` is empty. */
  in_terminal?: string;
  /** `in_terminal` is a regular expression. Absent is false. */
  regex?: boolean;
  /** Where its environment came from, while Fleet holds it: absent for a terminal watch, an ended one, and one not run since Fleet started. */
  login_shell?: LoginShell;
};

/** Where a watch's environment came from: the person's login shell, or Fleet's own and why. */
export type LoginShell = { kind: "read" } | { kind: "fell_back"; why: string };

/**
 * One `weigh_watch`, as kept on the watch. `at` places it after the reading it weighed. Each verdict carries the agent's
 * `sentence`; `parts` are those a `not_ours` set aside.
 */
export type WatchVerdict = { at: string; verdict: "ours" | "not_ours"; sentence: string; parts?: string[] };

/** `list_watches`: a Job's live watches, and with `?ended=true` its ended ones too, the newest first. `watches.changed` carries both. */
export type WatchList = { session_id: string; watches: WatchSummary[] };

/** `watches.changed`: one Job's watches after any change to one, whole, as `list_watches` answers for it. */
export type WatchesChanged = WatchList;

/** `start_watch`: a recipe with its values, the agent's own command with `every`, or text to wait for in the Job's terminal. */
export type StartWatch = {
  session_id?: string;
  name: string;
  follows?: number;
  set_aside?: WatchSetAside[];
} & (
  | { recipe: string; values?: Record<string, string> }
  | { command: string[]; every: string; gives_up_after?: string }
  | { in_terminal: string; regex?: boolean; gives_up_after?: string }
);

/** `try_watch`: a recipe with its values, or the agent's own command, run once and kept nowhere. */
export type TryWatch = { session_id?: string } & ({ recipe: string; values?: Record<string, string> } | { command: string[] });

/** What `try_watch` read, and where and how its command ran. */
export type WatchTried = {
  reading: WatchReading;
  command: string[];
  /** The Job's slot. Empty where its folder is gone or the Job ended. */
  cwd: string;
  /** The `PATH` the command was given. */
  path: string;
  login_shell: LoginShell;
  /** The Job's own scratch folder. */
  scratch: string;
};

/** Where a recipe was read from. */
export type WatchRecipeSource = "armada" | "repository" | "machine";

export type WatchRecipeListed = {
  name: string;
  /** The values a start gives, in the order its command names them. */
  values: string[];
  command: string[];
  source: WatchRecipeSource;
  every_seconds: number;
  /** Absent gives up after the machine's setting. */
  gives_up_after_seconds?: number;
  /** A Requests environment's name, never a value. */
  environment?: string;
};

/** `list_watch_recipes`: one per name, the copy a start would run, and each file left out with why. */
export type WatchRecipeList = { recipes: WatchRecipeListed[]; left_out?: string[] };

/** `save_watch_recipe`: written into this Mac's own recipes only. */
export type SaveWatchRecipe = {
  session_id?: string;
  name: string;
  /** A script's whole path, copied beside the recipe and run first. */
  script?: string;
  /** The whole command, or with `script` the arguments after it. */
  command?: string[];
  every: string;
  gives_up_after?: string;
  environment?: string;
  replace?: boolean;
};

export type WatchRecipeSaved = { recipe: WatchRecipeListed; file: string; script?: string; replaced: boolean };

/** `weigh_watch`, each verdict with its sentence. `ours` ends the watch failed; `not_ours` sets its parts aside. */
export type WeighWatch = { session_id?: string; watch: string } & (
  | { verdict: "ours"; sentence: string }
  | { verdict: "not_ours"; sentence: string; parts: string[] }
);

/** `stop_watch`: the person's Stop, or the agent's. */
export type StopWatch = { session_id?: string; watch: string };

/** `delete_watch`: an ended watch only, and all kept of it. Answers the Job's `WatchList` after, live and ended. */
export type DeleteWatch = { session_id?: string; watch: string };
