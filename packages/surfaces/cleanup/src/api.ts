// Cleanup: what Fleet holds disk for, the worktree pool.
// Types and values only, no React. A slice imports protocol and screens, never another slice;
// desktop's `shared/api/cleanup.ts` re-exports it and `shared/api.ts` composes it.

import type {
  Outcome,
  ChangeSlotPool,
  RescueSlot,
  HeldWorktrees,
} from "@armada/protocol";
import type { RescueOutcome } from "./slot-rescue";

export type * from "./slot-rescue";

export type CleanupApi = {
  /**
   * Add, remove, close or reopen one slot of a repository's worktree pool, on
   * this machine. Cleanup is read again after, whatever came back.
   */
  changeSlotPool: (manifestId: string, change: ChangeSlotPool) => Promise<Outcome>;
  /**
   * Start or stop a rescue Scout on a stranded slot, or Scrap or Stash what it
   * holds. Cleanup is read again after, whatever came back; the receipt is the
   * one thing that read cannot carry.
   */
  rescueSlot: (manifestId: string, rescue: RescueSlot) => Promise<RescueOutcome>;
  /**
   * Read what Fleet is holding disk for, or drop it.
   *
   * **Read-only.** What comes back is every slot of the pool and what holds it.
   *
   * A boolean rather than an id: there is nothing to scope it to, only
   * whether somebody is looking.
   */
  readHeld: (want: boolean) => Promise<void>;
};

export type CleanupState = {
  /**
   * What Fleet is holding disk for, where a surface asked.
   *
   * Read when the surface opens and dropped when it closes.
   */
  held: HeldWorktrees;
};

export const CLEANUP_NOTHING_YET: CleanupState = {
  held: { state: "none" },
};

export const CLEANUP_CHANNELS = {
  changeSlotPool: "bridge:change-slot-pool",
  rescueSlot: "bridge:rescue-slot",
  readHeld: "bridge:read-held",
} as const;
