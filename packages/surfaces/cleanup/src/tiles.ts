// Which worktrees are a bay's and which stand outside the pool, for the grid
// Cleanup draws. Its own file because it is arithmetic and is unit-tested.

import type { WorktreeHeld, WorktreeSlot } from "@armada/protocol";

export type Joined = {
  /** Every slot, with its Job's worktree reading where it has one. */
  bays: { slot: WorktreeSlot; held?: WorktreeHeld }[];
  /** Worktrees no slot holds, in Fleet's order. */
  outside: WorktreeHeld[];
};

/**
 * Join each slot to its Job's reading by job id, so the bay carries its slot
 * acts and its reclaim acts together. **A reading at a slot's path joins that
 * slot too**, so a worktree whose Job no longer holds the bay is not lost: it
 * is outside the pool only where no slot is its Job's and none is at its path.
 */
export function joined(slots: readonly WorktreeSlot[], worktrees: readonly WorktreeHeld[]): Joined {
  const used = new Set<string>();
  const bays = slots.map((slot) => {
    const jobId = slot.held.state === "job" ? slot.held.job_id : undefined;
    const found = worktrees.find((one) => one.job_id === jobId) ?? worktrees.find((one) => one.path === slot.path);
    if (found === undefined) return { slot };
    used.add(found.job_id);
    return { slot, held: found };
  });
  return { bays, outside: worktrees.filter((one) => !used.has(one.job_id)) };
}
