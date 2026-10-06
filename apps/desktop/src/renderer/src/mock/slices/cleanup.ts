// Cleanup's members: reclaiming worktrees, the slot pool, and forgetting Jobs.

import type { Outcome } from "@armada/protocol";
import type { RescueOutcome } from "@armada/screens/src/slot-rescue";

import type { CleanupApi, CleanupState } from "../../../../shared/api/cleanup";
import { CLEANUP_NOTHING_YET } from "../../../../shared/api/cleanup";
import type { Slice } from "../fake-context";
import { nothing, proposingRow } from "../fake-context";
import { branchDeletedIn, forgottenIn, reclaimedIn } from "../cleanup-fleet";
import { reshaped, rescued, scoutRead } from "../slots-fleet";
import { unanswered } from "../moment";

const OK: Outcome = { ok: true };

export const cleanup: Slice<CleanupApi, CleanupState> = {
  name: "cleanup",
  state: CLEANUP_NOTHING_YET,
  api: (_scenario, { state, publish, move, held: pool, unread }) => {
    function forget(jobIds: readonly string[]): void {
      publish({ jobs: state().jobs.filter((job) => !jobIds.includes(job.id)) });
    }
    /** The pool as an act left it, published where the window is reading it. */
    const settled = (held: NonNullable<ReturnType<typeof pool.get>>): void => {
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
          held: !want ? nothing : held === undefined ? unread("/worktrees/held") : { state: "read", held },
        });
      },
    };
  },
};
