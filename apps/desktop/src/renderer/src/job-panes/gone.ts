// What Bridge keeps of a Job once the Job is gone: nothing. A Job is gone when Fleet's list no longer has it, or when it ended more than
// GONE_AFTER_MS ago. The sweep runs only on a list Fleet answered whole (`listedAt`), at most once every SWEEP_EVERY_MS, and never drops a Job drawn
// in the window. A list from disk, one held through a failed read, or an empty one is not a fact about any Job, so it drops nothing.

import type { SessionRecord } from "@armada/protocol";

import type { BridgeState } from "../../../shared/bridge";
import { forgetCustom, forgetOldCustomDrafts } from "./custom";
import { jobPanes } from "./store";

/** A week after a Job ends, its arrangement is let go. */
export const GONE_AFTER_MS = 7 * 24 * 60 * 60 * 1000;

/** The Job list arrives on every event; a Job crossing its week need not be dropped to the second. */
export const SWEEP_EVERY_MS = 10 * 60 * 1000;

/** Whether nothing of the Job need be kept. Live is never gone; ended with no `ended_at` is kept, since its age cannot be told. */
export function isGone(
  id: string,
  records: readonly Pick<SessionRecord, "id" | "state" | "ended_at">[],
  now: number,
): boolean {
  const record = records.find((r) => r.id === id);
  if (record === undefined) return true;
  if (record.state !== "ended" || record.ended_at === undefined) return false;
  const ended = Date.parse(record.ended_at);
  return !Number.isNaN(ended) && now - ended > GONE_AFTER_MS;
}

/** Everything Bridge keeps of one gone Job, dropped. The one place a Job's other state is let go with its panes. */
function dropJob(id: string): void {
  jobPanes.drop(id);
  forgetCustom(id);
}

/** Drops every gone Job that holds panes and is not drawn now, and says which. */
export function sweepGoneJobs(
  records: readonly Pick<SessionRecord, "id" | "state" | "ended_at">[],
  now: number,
): string[] {
  const dropped = jobPanes
    .ownerIds()
    .filter((id) => isGone(id, records, now) && !jobPanes.isDrawn(id));
  dropped.forEach(dropJob);
  return dropped;
}

let sweptAt: number | undefined;
let oldDraftsForgotten = false;

/**
 * Sweeps on a list Fleet answered whole, once per SWEEP_EVERY_MS. The picture from disk arrives as read too, and so does the connection's `readAt` before
 * the list is asked for, so only the list's own `listedAt` says Fleet answered it. An empty answer cannot be told from a ledger that failed to load.
 */
export function sweepOnRead(sessions: BridgeState["sessions"], now: number): string[] {
  if (
    sessions.state !== "read" ||
    sessions.listedAt === undefined ||
    sessions.sessions.length === 0
  )
    return [];
  if (sweptAt !== undefined && now - sweptAt < SWEEP_EVERY_MS) return [];
  sweptAt = now;
  // Drafts kept before they named their Job belong to no Job to sweep; once per window is enough.
  if (!oldDraftsForgotten) {
    oldDraftsForgotten = true;
    forgetOldCustomDrafts();
  }
  return sweepGoneJobs(sessions.sessions, now);
}

/** The throttle as a window opens with it: a test starts from here. */
export function resetSweep(): void {
  sweptAt = undefined;
  oldDraftsForgotten = false;
}
