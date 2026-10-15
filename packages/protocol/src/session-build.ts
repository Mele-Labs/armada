// What a Session has built, mirrored by hand from `crates/ipc/src/session_build.rs`.
// Observed by Fleet from edits, commits, pushes, walks and the forge; the agent's task list only adds detail.
// Derived on every read and carried on `SessionRecord.build`, so no figure is held beside the facts it is read from.

/**
 * Where the Session stands on the way from nothing to main. `scope`: no edit seen and no plan. `plan`: a task list
 * written or plan mode left, and no edit since. `implement`: edits seen. `road`: a stop of the road reached after the
 * first edit. `landed`: the Session's pull request merged.
 */
export type BuildPhase = "scope" | "plan" | "implement" | "road" | "landed";

/**
 * How much of one task is built. `solid`: completed, and an edit was seen while it was worked. `hollow`: completed with
 * no edit seen, claimed and not observed. `scaffold`: the first task not completed, while edits have landed since the
 * last completion. `ghost`: every other task not completed.
 */
export type BuildPieceKind = "solid" | "hollow" | "scaffold" | "ghost";

/**
 * One task of the agent's list, in list order. `id` is its task tool's, or its place in a `TodoWrite` list. `by` is the
 * subagent that wrote the task, in the form `SessionBuild.workers` uses (`Type: what it was asked`), so a piece is
 * drawn with its worker; absent for the Session's own tasks. It stays after the subagent ends.
 */
export type BuildPiece = { id: string; text: string; piece: BuildPieceKind; by?: string };

/**
 * A place on the road from the Session's edits to main: a walk shown after the first edit, a `git push` that updated a
 * ref, the pull request opened, its checks coming to a state, auto-merge set or the pull request queued. Landing is
 * not one: it closes the road, as `SessionBuild.landed_at`.
 */
export type BuildStopId = "walk" | "pushed" | "pr" | "checks" | "auto_merge";

/** One stop of the road. `at` is absent where it has not been reached, and the stop is a forecast, drawn faint. */
export type BuildStop = { id: BuildStopId; at?: string };

/** Carried on `SessionRecord.build`. Absent from the row of a Session that has done nothing Fleet observed. */
export type SessionBuild = {
  phase: BuildPhase;
  /** One per task, in list order. Empty where the agent wrote no list: progress is never invented. */
  pieces: BuildPiece[];
  /**
   * The road, ready to draw: the stops reached in the order they were, then the ones the repository's recent landed
   * Sessions usually reached and this one has not, each with no `at`.
   */
  stops: BuildStop[];
  /**
   * The stops the repository's recent landed Sessions usually reached, in order, reached or not: the most common whole
   * sequence among its last ten. Empty where it has no history.
   */
  forecast: BuildStopId[];
  /** When the pull request merged. The road closes here. */
  landed_at?: string;
  /** The last tool activity Fleet observed, as of the last time the row went out. Bridge dims the unfinished part by how long ago. */
  quiet_since?: string;
  /** The Session's `standing` is `stuck`. */
  stalled: boolean;
  /** The subagents still running, each as `Type: what it was asked`. */
  workers: string[];
};

/** Where one task of the agent's list stands, as its tool says. */
export type ReportedTaskStatus = "pending" | "in_progress" | "completed";

/**
 * One task of the list a mod heard the agent write. The mod sends the whole list on every change. `by` is the
 * `agentId` of the subagent that wrote it: subagents share the Session's task list. Absent for the Session's own.
 */
export type ReportedTask = { id: string; text: string; status: ReportedTaskStatus; by?: string };
