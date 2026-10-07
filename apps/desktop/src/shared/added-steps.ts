// What Bridge asks the host for about steps added to one Job, and what it is answered with.
// `main/added-steps.ts` builds these from Fleet's two routes; nothing else does.

import type { AddedStep, AddedStepRemoved, AddStep, Outcome, RemoveAddedStep } from "@armada/protocol";

/** `POST /jobs/:job_id/add_job_step`. A refusal carries Fleet's own sentence and code on `outcome`. */
export type AddStepAnswer = { ok: true; added: AddedStep } | { ok: false; outcome: Outcome };

/** `POST /jobs/:job_id/remove_job_step`. */
export type RemoveStepAnswer = { ok: true; removed: AddedStepRemoved } | { ok: false; outcome: Outcome };

/** One step to add to a Job that is underway. */
export type AddingStep = { jobId: string; body: AddStep };

/** One step to take off, before it fires. */
export type RemovingStep = { jobId: string; body: RemoveAddedStep };
