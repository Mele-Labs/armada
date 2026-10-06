// `park_job` and `resume_job` as Fleet answers them, over the Board's rows and
// the pool: a running Job reads queued and paused with its slot freed, a Job at
// a gate keeps its status and gains the marker, and Resume seats it again where
// a slot is free. Acts on a paused Job are refused `fleet.paused`.

import type { JobSummary, Outcome, WorktreesHeld } from "@armada/protocol";

const WIP_COMMIT = "7e2a90c4d1f3";

const refusedAs = (code: string, message: string): Outcome => ({
  ok: false,
  why: "refused",
  error: { code, message, run_id: "mock", fields: {}, chain: [] },
});

const PAUSABLE = ["running", "awaiting_review", "awaiting_repair", "escalated"];

/** What an act on a Job is answered, where the Job is paused. `undefined` where it is not. */
export function pausedRefusal(job: JobSummary | undefined): Outcome | undefined {
  return job?.paused === undefined ? undefined : refusedAs("fleet.paused", "the job is paused: resume it first");
}

export type Parked = { job: JobSummary; held: WorktreesHeld | undefined };

/** The bay a Job holds, if it holds one. */
const bayOf = (held: WorktreesHeld | undefined, jobId: string) =>
  held?.slots?.find((one) => one.held.state === "job" && one.held.job_id === jobId);

/** `park_job`: the work is committed to the branch, the slot goes back, and the Job carries the marker. */
export function parked(job: JobSummary, held: WorktreesHeld | undefined, at: string): Parked | Outcome {
  if (job.paused !== undefined) return refusedAs("fleet.already_paused", "the job is already paused");
  if (!PAUSABLE.includes(job.status)) return refusedAs("fleet.not_pausable", "the job holds no worktree to give back");
  const bay = bayOf(held, job.id);
  const after: JobSummary = {
    ...job,
    paused: { by: "person", at, resuming: false },
    ...(job.status === "running" ? { status: "queued", queued_reason: "paused" } : {}),
  };
  if (held === undefined || bay === undefined) return { job: after, held };
  const { branch: _b, since: _s, ...rest } = bay;
  const freed = { ...rest, held: { state: "free" as const } };
  const saved = (row: WorktreesHeld["worktrees"][number]) => {
    if (row.job_id !== job.id) return row;
    const dirty = row.held.some((reason) => reason.why === "uncommitted");
    const unmerged = row.held.find((reason) => reason.why === "unmerged");
    const standing = dirty
      ? [{ why: "unmerged" as const, base: unmerged?.base ?? "main", commits: (unmerged?.commits ?? 0) + 1, tip: WIP_COMMIT }]
      : row.held.filter((reason) => reason.why === "unmerged");
    return { ...row, status: after.status, held: standing };
  };
  return {
    job: after,
    held: { ...held, slots: (held.slots ?? []).map((one) => (one === bay ? freed : one)), worktrees: held.worktrees.map(saved) },
  };
}

/** `resume_job`: a gate Job takes a free slot at once, and waits where none is free. */
export function resumed(job: JobSummary, held: WorktreesHeld | undefined): Parked | Outcome {
  if (job.paused === undefined) return refusedAs("fleet.not_paused", "the job is not paused");
  const { paused, ...rest } = job;
  const free = held?.slots?.find((one) => one.held.state === "free" && one.closed !== true);
  const gate = job.status !== "queued";
  if (gate && held !== undefined && free === undefined) return { job: { ...job, paused: { ...paused, resuming: true } }, held };
  // A Job that was working goes back in the line, and admission starts it.
  const { queued_reason: _q, ...line } = rest;
  const after: JobSummary = gate ? rest : line;
  if (!gate || held === undefined || free === undefined) return { job: after, held };
  const seated = { ...free, held: { state: "job" as const, job_id: job.id, job_title: job.title }, branch: job.branch ?? free.branch, warm: true };
  return { job: after, held: { ...held, slots: (held.slots ?? []).map((one) => (one === free ? seated : one)) } };
}

/** Whether an answer is a refusal rather than the Job moved. */
export const isOutcome = (answer: Parked | Outcome): answer is Outcome => "ok" in answer;
