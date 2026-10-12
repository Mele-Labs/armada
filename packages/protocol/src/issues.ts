// What is wrong with a Job, and what a quiet one is waiting on: the two lists a
// Job's Now draws under Issues and Waiting. `crates/ipc/src/issues.rs`.
//
// **Pointers, not copies.** An issue names the thing it is about and the reader
// finds the rest where it already lives: `JobDetail.breakages`,
// `write_scope_overlaps`, the step's verdict.

/** One thing wrong with a Job right now, once each. Tagged by `kind`. */
export type JobIssue =
  /** A test broken on main this Job reported or is fixing; the claim is in `JobDetail.breakages`. */
  | { kind: "breakage"; check: string; test: string }
  /** Another Job claiming to write where this one does; the paths are in `write_scope_overlaps`. */
  | { kind: "write_scope_overlap"; job_id: string }
  /** The latest ruling on a step was `failed`, with the trigger it carried. */
  | { kind: "failed_verdict"; step_id: string; attempt: number; trigger?: string }
  /** A working Drone that has written nothing for longer than Fleet's quiet threshold. */
  | { kind: "stuck_drone"; drone_id: string; step_id: string; quiet_since: string };

/** What a Job with nothing running is waiting on. Tagged by `kind`. */
export type JobWaiting =
  /** A named resource, spelled as `JobSummary.queued_reason` is: rendered, never matched on. */
  | { kind: "resource"; name: string }
  /** Another Job it depends on that has not ended. */
  | { kind: "job"; job_id: string; title?: string }
  /** A step between two states, with no Drone on it yet. */
  | { kind: "transition"; step_id: string };
