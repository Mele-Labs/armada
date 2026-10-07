// Taking a Job over, and the bundle a person is handed when they do.
// `crates/ipc/src/piloting.rs`, `docs/concepts/pilot.md`. Since protocol 23.49.
//
// The header rules in `protocol.ts` hold here: hand-written, and every closed
// set is left as `string`.

import type { JobDetail } from "./detail";
import type { ChangedFile } from "./events";
import type { DeclaredPlan } from "./footprint";
import type { JobHistory } from "./history";
import type { JobEvidence } from "./work";

/**
 * What a person asked of the job they are taking over. **Assist is not an
 * outcome**: it is deferred, and a body naming it does not decode.
 */
export type PilotOutcome = "take_over" | "restart_step";

/**
 * The body of `take_over`, `POST /jobs/:job_id/take_over`, or none for a take
 * over with no session named.
 */
export type TakeOver = {
  outcome: PilotOutcome;
  /** The session they pilot from, so the Board can say who is in the worktree. Fleet does not start it. */
  session_id?: string;
};

/** The body of `attest_complete` and `close_as_superseded`. Never required. */
export type PilotNote = {
  note?: string;
};

/**
 * A job's pilot on its row, **beside the status and never instead of it**.
 * Present on a `piloted` job and still present once the pilot ended, because
 * `exit` is what tells a job a person attested from one that passed its gates.
 * `JobSummary.piloted`.
 */
export type Piloted = {
  /** `take_over` or `restart_step`. */
  reason: string;
  /** The session in the worktree, where one was named. */
  session_id?: string;
  since: string;
  /**
   * `submitted`, `attested` or `superseded`. Absent while the person is still
   * working. **`attested` is never verified**, and is drawn as what it is.
   */
  exit?: string;
  ended_at?: string;
  /** The person's words on an attestation or a supersede. */
  note?: string;
};

/** What the Drone said it was stuck on. Not evidence: it states that no proof is coming. */
export type DroneNarrative = {
  trying_to: string;
  blocked_by: string;
  tried: string;
};

/** The step a job stopped on, and the trigger it stopped under. */
export type StoppedOn = {
  step_id: string;
  trigger?: string;
};

/** Where the person works. `path` is a path on the machine Fleet runs on. */
export type HandoffWorktree = {
  path: string;
  branch: string;
};

/**
 * `get_handoff`, `GET /jobs/:job_id/handoff`: everything a person, or a session
 * on their behalf, needs to pick a job up. 409 `fleet.not_piloted` on a job
 * nobody took over. Served after the pilot ended too.
 */
export type HandoffBundle = {
  /** The job whole: its steps, their checks and the judge's refusals, and each step's verdict. */
  job: JobDetail;
  /** Every move the job made, so every attempt of every step is in it. */
  history: JobHistory;
  /** The evidence each step's Drone submitted. */
  evidence: JobEvidence;
  /** `take_over` or `restart_step`. */
  reason: string;
  session_id?: string;
  worktree?: HandoffWorktree;
  stopped_on?: StoppedOn;
  /** What each run of each step said its work would be. */
  plans: DeclaredPlan[];
  /** False is no plan, not nothing drifted. */
  plan_declared: boolean;
  /** The worktree as it stands, each file marked where it is outside the plan. */
  changed: ChangedFile[];
  narrative?: DroneNarrative;
};
