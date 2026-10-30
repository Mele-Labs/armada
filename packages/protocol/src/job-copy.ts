// A Job's own copy of its workflow, lifecycles and rules, and the changes made to it.
// Hand-mirrored from `crates/ipc/src/job_copy.rs`. `docs/practices/protocol.md`, *A Job's own copy*.
//
// **A change applies to the one Job and to nothing else.** `keep_job_workflow` is what proposes it for every Job, and it is saved only on the
// owner's Approve. Every closed set is left as `string`, with its words in the comment.

import type { PieceScope } from "./lifecycles";

/** Who made a change to a Job's copy, as a word: `job` is the Job's agent, with a tool; `you` is the owner, from the Job's lanes. */
export type ChangedBy = string;

/** One edit to the steps of a Job's copy of its workflow, tagged by `kind`. A step is named by its id. */
export type StepChange =
  | { kind: "add"; after?: string; id?: string; label: string; about?: string; instructions?: string; ends_on?: string[] }
  | { kind: "move"; step: string; after?: string }
  | { kind: "reword"; step: string; label?: string; about?: string; instructions?: string }
  | { kind: "drop"; step: string };

/** `change_session_steps` (`POST /sessions/workflow/steps`). Answers the Job's `JobCopy`. */
export type ChangeSessionSteps = { session_id: string; change: StepChange };

/**
 * `change_job_copy` (`POST /sessions/copy`): a lifecycle or a rule put in the Job's copy, or taken out of it with no `text`. `kind` is `lifecycle` or `rule`.
 * `id` is the one the file declares, and may be blank where `text` is given.
 */
export type ChangeJobCopy = { session_id: string; kind: string; id: string; text?: string };

/** One step of a Job's workflow as it holds it. `ends_on` is one word or lifecycle state each, as the file writes them. */
export type JobCopyStep = { id: string; label: string; about?: string; instructions?: string; ends_on?: string[]; skipped?: boolean };

/** A lifecycle or a rule the Job's copy holds, as the file it was kept from says it. */
export type JobCopyPiece = { id: string; text: string };

/**
 * One change to the Job's copy that the lanes mark. `lane` is `workflow` with a step as `item`, `rule` with a rule's id, or a lifecycle's
 * id with no `item`. `text` names who made it: `Added after Plan the change by the Job`.
 */
export type JobCopyChange = { lane: string; item: string; text: string; by: ChangedBy; at: string; /** A rule or a lifecycle the Job took out of its copy: nothing of it is left to keep for every Job. */ dropped?: boolean };

/** `get_job_copy` (`GET /sessions/copy?session_id=`): what a Job holds of its workflow, lifecycles and rules, and what was changed in it. */
export type JobCopy = {
  session_id: string;
  /** The workflow the Job follows. Absent for a custom Job. */
  workflow?: string;
  /** The steps are the Job's own, because it changed one. Otherwise they are the workflow as the repository holds it now. */
  own?: boolean;
  steps: JobCopyStep[];
  lifecycles: JobCopyPiece[];
  rules: JobCopyPiece[];
  changes: JobCopyChange[];
};

/**
 * `keep_job_piece` (`POST /sessions/copy/keep_piece`): one rule or lifecycle the Job changed in its copy, proposed for every Job as a file in Kit within the Job's
 * repository. `kind` is `lifecycle` or `rule`, and `id` is the one the piece declares. Answers a `JobPieceProposed`, which waits on the owner's Approve.
 */
export type KeepJobPiece = { session_id: string; kind: string; id: string };

/** What `keep_job_piece` kept: a proposal on the owner's Needs-you list, and nothing written. `replaces` is whether Kit holds a file for the piece already. */
export type JobPieceProposed = { change: number; kind: string; id: string; scope: PieceScope; replaces: boolean };

/** `keep_job_workflow` (`POST /sessions/copy/keep`): the Job's copy of its workflow proposed for every Job. `source` is `repository` or `kit`. Answers a `WorkflowChange`. */
export type KeepJobWorkflow = { session_id: string; source?: string };
