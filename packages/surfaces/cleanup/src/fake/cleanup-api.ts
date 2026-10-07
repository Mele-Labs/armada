// Cleanup's members as a mock Fleet answers them: reclaiming worktrees, the slot pool, and
// forgetting Jobs, over the pool the scenario holds.

import { unanswered } from "@armada/bridge-api";
import type { FleetHandle } from "@armada/bridge-api";
import type { JobSummary, Outcome, WorktreesHeld } from "@armada/protocol";

import { CLEANUP_NOTHING_YET } from "../api";
import type { CleanupApi, CleanupState } from "../api";
import type { RescueOutcome } from "../slot-rescue";
import { branchDeletedIn, forgottenIn, reclaimedIn } from "./cleanup-fleet";
import { reshaped, rescued, scoutRead } from "./slots-fleet";

const OK: Outcome = { ok: true };

/** The state Cleanup's members read and write: the Board's Jobs, and what the pool holds. */
export type CleanupHeld = CleanupState & { jobs: JobSummary[] };

/**
 * What Cleanup's members reach beyond the state: one field on one Job, the pool as acts on it
 * left it, the read that failed, and the row a Pick up proposes. The app's fake supplies them.
 */
export type CleanupFleet = FleetHandle<CleanupHeld> & {
  move: (jobId: string, change: Partial<JobSummary>) => void;
  held: { get: () => WorktreesHeld | undefined; set: (held: WorktreesHeld | undefined) => void };
  unread: (route: string) => { state: "failed"; outcome: Outcome };
  proposingRow: (request: string, at: string, jobs: readonly JobSummary[]) => JobSummary;
};

export function cleanupApi({ state, publish, move, held: pool, unread, proposingRow }: CleanupFleet): CleanupApi {
  function forget(jobIds: readonly string[]): void {
    publish({ jobs: state().jobs.filter((job) => !jobIds.includes(job.id)) });
  }
  /** The pool as an act left it, published where the window is reading it. */
  const settled = (held: WorktreesHeld): void => {
    pool.set(held);
    if (state().held.state === "read") publish({ held: { state: "read", held } });
  };
  return {
    clearTerminalJobs: async (jobIds) => {
      const at = new Date().toISOString();
      jobIds.forEach((jobId) => move(jobId, { reclaimed_at: at }));
      return { reclaimed: [], failed: [] };
    },
    forgetTerminalJobs: async (jobIds) => (forget(jobIds), { cleared: [...jobIds], failed: [] }),
    reclaimWorktree: async (jobId) => {
      const held = pool.get();
      const after = held === undefined ? undefined : reclaimedIn(held, jobId);
      if (after === undefined) return (move(jobId, { reclaimed_at: new Date().toISOString() }), OK);
      settled(after.held);
      return after.outcome;
    },
    changeSlotPool: async (manifestId, change) => {
      const held = pool.get();
      if (held === undefined) return unanswered(`/worktrees/slots?manifest_id=${manifestId}`);
      const after = reshaped(held, manifestId, change);
      settled(after.held);
      return after.outcome;
    },
    rescueSlot: async (manifestId, rescue) => {
      const held = pool.get();
      if (held === undefined) return unanswered(`/worktrees/slots/rescue?manifest_id=${manifestId}`) as RescueOutcome;
      const after = rescued(held, manifestId, rescue);
      pool.set(after.held);
      // A Pick up proposes a Job from the press, as a dispatched request does.
      const proposing =
        after.proposed === undefined
          ? {}
          : { jobs: [...state().jobs, proposingRow(after.proposed.split("\n")[0] ?? "", new Date().toISOString(), state().jobs)] };
      publish(state().held.state === "read" ? { ...proposing, held: { state: "read", held: after.held } } : proposing);
      return after.outcome;
    },
    deleteBranch: async (jobId, tip) => {
      const held = pool.get();
      const after = held === undefined ? undefined : branchDeletedIn(held, jobId, tip);
      if (after === undefined) return OK;
      settled(after.held);
      return after.outcome;
    },
    forgetJob: async (jobId) => {
      forget([jobId]);
      const held = pool.get();
      if (held !== undefined) settled(forgottenIn(held, jobId));
      return OK;
    },
    readHeld: async (want) => {
      let held = pool.get();
      // A Scout reads between two reads, as Fleet's does: each one finds it a file further on.
      if (want && held !== undefined) pool.set((held = scoutRead(held)));
      publish({
        held: !want ? CLEANUP_NOTHING_YET.held : held === undefined ? unread("/worktrees/held") : { state: "read", held },
      });
    },
  };
}
