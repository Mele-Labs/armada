// Cleanup and Worktrees: what Fleet holds disk for, and giving it back.
// Types and values only, no React. A slice imports protocol and screens, never another slice;
// desktop's `shared/api/cleanup.ts` re-exports it and `shared/api.ts` composes it.

import type {
  ClearOutcome,
  Outcome,
  ReclaimOutcome,
  ChangeSlotPool,
  RescueSlot,
  HeldWorktrees,
} from "@armada/protocol";
import type { RescueOutcome } from "./slot-rescue";

export type * from "./slot-rescue";

export type CleanupApi = {
  /**
   * Reclaim every terminal Job's worktree and branch at once, one
   * `reclaim_worktree` per id. **Every row survives** — this takes the
   * directory and the branch, and `forgetTerminalJobs` below is the act that
   * takes the row. The caller decides which ids are terminal; this sends
   * exactly the ids it is given and refuses none of them itself.
   *
   * **A branch nothing has merged is kept**, always — there is no force here —
   * so a failed id in the outcome may still have given its disk back; read
   * `reclaimed.branch` on the per-Job act for which half happened.
   */
  clearTerminalJobs: (jobIds: readonly string[]) => Promise<ReclaimOutcome>;
  /**
   * Delete every terminal Job's whole record at once, one `forget_job` per
   * id. **Real deletion, not a status** — there is no undo, and a Job this
   * reaches cannot be opened again. The caller decides which ids are
   * terminal; this sends exactly the ids it is given and refuses none of them
   * itself.
   */
  forgetTerminalJobs: (jobIds: readonly string[]) => Promise<ClearOutcome>;
  /**
   * Give one terminal Job's worktree and branch back, **without waiting for
   * Fleet to stop**. `armada clean` is the same act from the CLI and refuses
   * while the daemon is running, which is exactly when a person wants the disk.
   *
   * **The record survives.** This takes the directory and the branch;
   * `forgetJob` takes the row. Sending both is ordinary and the order does
   * not matter.
   *
   * **A branch nothing has merged is kept**, always — there is no force here —
   * so the outcome's `reclaimed` says which half happened rather than reducing
   * both to one flag.
   */
  reclaimWorktree: (jobId: string) => Promise<Outcome>;
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
   * Delete one terminal Job's branch, sending the tip a person confirmed. **A
   * force** — Fleet refuses with 409 where the Job is not terminal, the
   * checkout is still on disk, the branch is gone, or the tip has moved.
   */
  deleteBranch: (jobId: string, tip: string) => Promise<Outcome>;
  /**
   * Delete one terminal Job's whole record. **Real deletion, not a status** —
   * there is no undo, and the Job cannot be opened again. The per-Job half of
   * `forgetTerminalJobs`.
   */
  forgetJob: (jobId: string) => Promise<Outcome>;
  /**
   * Read what Fleet is holding disk for, or drop it.
   *
   * **Read-only, and the reasons are the payload.** What comes back is every
   * worktree Fleet is holding and the test each one failed, so a person can
   * decide about them one at a time — Fleet has already given back everything
   * that passed all five, without being asked.
   *
   * **A piloted Job's checkout is not in it.** Fleet drops it before answering,
   * so there is nothing here to hide and nothing that could be drawn by
   * mistake: a person is at an unrestricted toolset in that directory.
   *
   * A boolean rather than an id, for `readReports`'s reason: there is nothing
   * to scope it to, only whether somebody is looking.
   */
  readHeld: (want: boolean) => Promise<void>;
};

export type CleanupState = {
  /**
   * What Fleet is holding disk for, where a surface asked.
   *
   * **The second read here no Job scopes**, and not for the reports' reason: a
   * report outlives the Job it names, while this is a question that only makes
   * sense of the set — which of these to give back. A field on a Job could
   * carry the reasons and could not carry the choice.
   *
   * Read when the surface opens and dropped when it closes, like the reports.
   */
  held: HeldWorktrees;
};

export const CLEANUP_NOTHING_YET: CleanupState = {
  held: { state: "none" },
};

export const CLEANUP_CHANNELS = {
  clearTerminalJobs: "bridge:clear-terminal-jobs",
  forgetTerminalJobs: "bridge:forget-terminal-jobs",
  reclaimWorktree: "bridge:reclaim-worktree",
  changeSlotPool: "bridge:change-slot-pool",
  rescueSlot: "bridge:rescue-slot",
  deleteBranch: "bridge:delete-branch",
  forgetJob: "bridge:forget-job",
  readHeld: "bridge:read-held",
} as const;
