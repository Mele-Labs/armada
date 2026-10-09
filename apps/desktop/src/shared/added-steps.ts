// What Bridge asks the host for about steps added to one Job, and what it is answered with.
// `main/added-steps.ts` builds these from Fleet's two routes; nothing else does.

import type { AddedStep, AddedStepRemoved, AddStep, EditAddedStep, JobDiff, Outcome, RemoveAddedStep, RepairOf } from "@armada/protocol";

/** `POST /jobs/:job_id/add_job_step`. A refusal carries Fleet's own sentence and code on `outcome`. */
export type AddStepAnswer = { ok: true; added: AddedStep } | { ok: false; outcome: Outcome };

/** `POST /jobs/:job_id/remove_job_step`. */
export type RemoveStepAnswer = { ok: true; removed: AddedStepRemoved } | { ok: false; outcome: Outcome };

/** `POST /jobs/:job_id/edit_job_step`: the row as it now stands. */
export type EditStepAnswer = { ok: true; edited: AddedStep } | { ok: false; outcome: Outcome };

/** One added step's switches to change, before it fires. */
export type EditingStep = { jobId: string; body: EditAddedStep };

/** `GET /jobs/:job_id/repair_diff`: the fix's patch against the Job's branch. */
export type RepairDiffAnswer = { ok: true; diff: JobDiff } | { ok: false; outcome: Outcome };

/** Which fix to read the diff of. */
export type ReadingRepairDiff = { jobId: string; of: RepairOf };

/** One step to add to a Job that is underway. */
export type AddingStep = { jobId: string; body: AddStep };

/** One step to take off, before it fires. */
export type RemovingStep = { jobId: string; body: RemoveAddedStep };
