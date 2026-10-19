// Who held one slot and what was checked out in it, for the side panel's Bay: read once for a
// repository and a slot. A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts`
// compose them.

import type { BayHistory, Outcome } from "@armada/protocol";

/** What reading a bay's history came to: Fleet's entries, newest first, or the refusal to word. */
export type BayHistoryRead = { ok: true; value: BayHistory } | { ok: false; outcome: Outcome };

export type BayHistoryApi = {
  /** `GET /slot_history`: the leases, releases and checkouts of slot `slot` of repository `manifestId`, newest first. A Fleet before it answers a refusal. */
  getSlotHistory: (manifestId: string, slot: number) => Promise<BayHistoryRead>;
};

export const BAY_HISTORY_CHANNELS = {
  getSlotHistory: "bridge:get-slot-history",
} as const;
