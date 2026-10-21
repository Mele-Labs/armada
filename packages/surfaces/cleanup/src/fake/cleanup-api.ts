// Cleanup's members as a mock Fleet answers them: the slot pool, over the pool the scenario holds.

import { unanswered } from "@armada/bridge-api";
import type { FleetHandle } from "@armada/bridge-api";
import type { Outcome, WorktreesHeld } from "@armada/protocol";

import { CLEANUP_NOTHING_YET } from "../api";
import type { CleanupApi, CleanupState } from "../api";
import type { RescueOutcome } from "../slot-rescue";
import { reshaped, rescued, scoutRead } from "./slots-fleet";

/**
 * What Cleanup's members reach beyond the state: the pool as acts on it left it, and the read
 * that failed. The app's fake supplies them.
 */
export type CleanupFleet = FleetHandle<CleanupState> & {
  held: { get: () => WorktreesHeld | undefined; set: (held: WorktreesHeld | undefined) => void };
  unread: (route: string) => { state: "failed"; outcome: Outcome };
};

/** How long a mock add holds its answer: a real worktree takes seconds, and a press that answers at once shows no wait. */
const ADD_TAKES_MS = 2500;

export function cleanupApi({ state, publish, held: pool, unread }: CleanupFleet): CleanupApi {
  /** The pool as an act left it, published where the window is reading it. */
  const settled = (held: WorktreesHeld): void => {
    pool.set(held);
    if (state().held.state === "read") publish({ held: { state: "read", held } });
  };
  return {
    changeSlotPool: async (manifestId, change) => {
      const held = pool.get();
      if (held === undefined) return unanswered(`/worktrees/slots?manifest_id=${manifestId}`);
      // Fleet makes the worktree before it answers an add, so the press waits as it does there.
      if (change.act === "add") await new Promise((resolve) => setTimeout(resolve, ADD_TAKES_MS));
      const after = reshaped(held, manifestId, change);
      settled(after.held);
      return after.outcome;
    },
    rescueSlot: async (manifestId, rescue) => {
      const held = pool.get();
      if (held === undefined) return unanswered(`/worktrees/slots/rescue?manifest_id=${manifestId}`) as RescueOutcome;
      const after = rescued(held, manifestId, rescue);
      settled(after.held);
      return after.outcome;
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
