// What Cleanup's grid draws a tile from, and what its panel offers on one.
// A tile is a bay of the pool, or a Job's worktree outside it; the screen joins
// them and decides which acts apply, because those rules are its own and are
// unit-tested there (`packages/screens/src/held.ts`).

import type { WorktreeHeld, WorktreeSlot } from "@armada/protocol";

/** Which of a worktree's reclaim acts apply: Clear, Delete branch, Forget Job. */
export type Offered = {
  clear: boolean;
  deleteBranch: boolean;
  forget: boolean;
  /** Pause a Job that holds this bay, and Resume one that is paused: the screen reads the Job. */
  pause?: boolean;
  resume?: boolean;
};

/** What Clear ends and leaves standing, named in its confirm. */
export type ClearCost = {
  /** Files written and committed nowhere: the checkout is their only copy. */
  files: readonly string[];
  /** An unmerged branch the reclaim keeps. */
  branch?: { name: string; commits: number; tip: string; base: string };
};

export type TileRow = {
  /** Unique across the grid. Absent is a bay's `manifest/slot`, or `held/` and the job id. */
  key?: string;
  /** Absent is `slot-4` for a bay; the Job's handle names a worktree outside the pool. */
  name?: string;
  /** Present for a bay, absent for a worktree outside the pool. */
  slot?: WorktreeSlot;
  /** The Job's handle, where the board knows it: what its status row names it by. */
  job?: string;
  /** The Job's worktree reading, joined to its bay by job id. */
  held?: WorktreeHeld;
  /** How long its holder has had the slot, formatted by the caller's clock. */
  heldFor?: string;
  /** How long the checkout has sat since Armada last moved its Job. */
  sat?: string;
  offered?: Offered;
  cost?: ClearCost;
  /** Why the last act on this tile was refused, said in its panel. */
  refused?: string;
  /** What the last rescue act did, as a bare fact: the branch a Scrap kept, the commit a Stash made. */
  said?: string;
  /** What the last Clear or Delete branch did, a fact to a line. */
  receipt?: readonly string[];
  /** The tooltip of the mark beside the Job's state, where its Job is paused. */
  paused?: string;
  /** An act on this tile is out, so its acts wait. */
  acting?: boolean;
};

export const keyOf = (row: TileRow): string =>
  row.key ?? (row.slot === undefined ? `held/${row.held?.job_id}` : `${row.slot.manifest_id}/${row.slot.slot}`);

export const nameOf = (row: TileRow): string =>
  row.name ?? (row.slot === undefined ? (row.held?.job_id ?? "") : `slot-${row.slot.slot}`);
