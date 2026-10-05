// Overview's Jobs placed on a time axis, for `JobTimeline`, and the state each was in at a moment.
//
// **Dispatched-from is `dispatched_by`**, the wire's own field (`JobSummary`, #1165): the parent's
// id, and the instant is the child's `created_at`, when Fleet minted it. `waits_on` is not it — a
// Job can wait on one it was not dispatched by — so it draws nothing here.
//
// **What the record holds is a start and an end, not every state between.** A Job is `queued`
// from `created_at` to `started_at`, `running` from there to `ended_at`, and its own `status`
// once ended. A Job still going shows `running` before now and its own status at now.

import type { JobTimelineBar, JobTimelineDispatch } from "@armada/components";
import type { JobSummary } from "@armada/protocol";
import { actsOf } from "./overview-graph";
import type { OverviewGraphActs } from "./overview-graph";
import { titleOf } from "./title";

const at = (iso: string | undefined): number | null => {
  if (iso === undefined) return null;
  const t = Date.parse(iso);
  return Number.isNaN(t) ? null : t;
};

/** The state `job` was in at `t`, or null before it existed. */
export function statusAt(job: JobSummary, t: number, now: number): string | null {
  const created = at(job.created_at) ?? 0;
  if (t < created) return null;
  const ended = at(job.ended_at);
  if (ended !== null && t >= ended) return job.status;
  if (ended === null && t >= now) return job.status;
  const started = at(job.started_at);
  return started === null || t < started ? "queued" : "running";
}

/** The Jobs as they stood at `t`: one not yet created is gone, the rest carry the state they were in. */
export function jobsAt(jobs: readonly JobSummary[], t: number, now: number): JobSummary[] {
  if (t >= now) return [...jobs];
  return jobs.flatMap((job) => {
    const status = statusAt(job, t, now);
    return status === null ? [] : [{ ...job, status }];
  });
}

export function timelineBarsOf(jobs: readonly JobSummary[], t: number, now: number, acts: OverviewGraphActs): JobTimelineBar[] {
  return jobs.map((job) => {
    const from = at(job.started_at) ?? at(job.created_at) ?? now;
    const to = at(job.ended_at) ?? now;
    return {
      id: job.id, card: { job: job.id, title: titleOf(job), handle: job.handle, onOpen: () => acts.onOpen(job.id), acts: actsOf(job, acts) },
      status: statusAt(job, t, now), from, to: Math.max(from, to) };
  });
}

/** Every Job whose dispatcher is also on the board. */
export function timelineDispatchesOf(jobs: readonly JobSummary[]): JobTimelineDispatch[] {
  const held = new Set(jobs.map((job) => job.id));
  return jobs.flatMap((job) => {
    const created = at(job.created_at);
    return job.dispatched_by !== undefined && job.dispatched_by !== job.id && held.has(job.dispatched_by) && created !== null
      ? [{ parent: job.dispatched_by, child: job.id, at: created }]
      : [];
  });
}
