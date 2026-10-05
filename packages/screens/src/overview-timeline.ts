// Overview's Jobs placed on a time axis, for `JobTimeline`, and the state each was in at a moment.
//
// **Dispatched-from is `dispatched_by`**, the wire's own field (`JobSummary`, #1165): the parent's
// id, and the instant is the child's `created_at`, when Fleet minted it. `waits_on` is not it — a
// Job can wait on one it was not dispatched by — so it draws nothing here.
//
// **What the record holds is a start and an end, not every state between.** A Job is `queued`
// from `created_at` to `started_at`, `running` from there to `ended_at`, and its own `status`
// once ended. A Job still going shows `running` before now and its own status at now.

import type { JobTimelineBar, JobTimelineDispatch, JobTimelineFamily } from "@armada/components";
import type { JobSummary } from "@armada/protocol";
import { actsOf } from "./overview-acts";
import type { OverviewActs } from "./overview-acts";
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

export function timelineBarsOf(jobs: readonly JobSummary[], t: number, now: number, acts: OverviewActs): JobTimelineBar[] {
  return jobs.map((job) => {
    const from = at(job.started_at) ?? at(job.created_at) ?? now;
    const to = at(job.ended_at) ?? now;
    return {
      id: job.id, card: { job: job.id, title: titleOf(job), handle: job.handle, onOpen: () => acts.onOpen(job.id), acts: actsOf(job, acts) },
      status: statusAt(job, t, now), from, to: Math.max(from, to) };
  });
}

/** The dispatcher of `job` where it is on the board and is not the Job itself. */
const dispatcherOf = (job: JobSummary, held: ReadonlySet<string>): string | undefined =>
  job.dispatched_by !== undefined && job.dispatched_by !== job.id && held.has(job.dispatched_by) ? job.dispatched_by : undefined;

const byCreated = (a: JobSummary, b: JobSummary) => (at(a.created_at) ?? 0) - (at(b.created_at) ?? 0) || a.id.localeCompare(b.id);

/**
 * The Jobs as families: a root, with every Job it dispatched, however deep, in the order they were
 * minted. A root is a Job nobody on the board dispatched: its dispatcher may have left the board.
 * A Job with no dispatcher and nothing it dispatched is alone, and belongs to no family.
 *
 * **Every Job lands in one family or in alone, even round a cycle.** `dispatched_by` should never
 * loop, but a board that holds a loop still draws: once the roots are spent, the earliest Job not
 * yet placed is taken as a root, and its own dispatcher is ignored so no edge is drawn into it.
 * `dispatches` is the edges the families drew, at each child's creation.
 */
export function familiesOf(jobs: readonly JobSummary[]): {
  families: JobTimelineFamily[];
  alone: string[];
  dispatches: JobTimelineDispatch[];
} {
  const held = new Set(jobs.map((job) => job.id));
  const sorted = [...jobs].sort(byCreated);
  const children = new Map<string, JobSummary[]>();
  for (const job of sorted) {
    const parent = dispatcherOf(job, held);
    if (parent !== undefined) children.set(parent, [...(children.get(parent) ?? []), job]);
  }
  const placed = new Set<string>();
  const grown: JobTimelineFamily[] = [];
  const dispatches: JobTimelineDispatch[] = [];
  const grow = (root: JobSummary) => {
    const members: string[] = [];
    const queue = [root];
    placed.add(root.id);
    for (let next = queue.shift(); next !== undefined; next = queue.shift()) {
      members.push(next.id);
      for (const child of children.get(next.id) ?? []) {
        if (placed.has(child.id)) continue;
        placed.add(child.id);
        dispatches.push({ parent: next.id, child: child.id, at: at(child.created_at) ?? 0 });
        queue.push(child);
      }
    }
    grown.push({ root: root.id, members });
  };
  for (const job of sorted) if (dispatcherOf(job, held) === undefined) grow(job);
  for (const job of sorted) if (!placed.has(job.id)) grow(job);
  return {
    families: grown.filter((one) => one.members.length > 1),
    alone: grown.filter((one) => one.members.length === 1).map((one) => one.root),
    dispatches,
  };
}
