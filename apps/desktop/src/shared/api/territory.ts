// What each served repository holds and where each agent has changed it: the directories the flight
// deck's map is drawn in, and the paths a Job or Session holding a slot has changed. A slice imports
// protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { Outcome, RepositoryAreas, SlotTouches } from "@armada/protocol";

/** What reading the areas came to: every served repository's, or the refusal to word. */
export type RepositoryAreasRead = { ok: true; value: RepositoryAreas } | { ok: false; outcome: Outcome };

/** What reading the touches came to: every holder's, or the refusal to word. */
export type SlotTouchesRead = { ok: true; value: SlotTouches } | { ok: false; outcome: Outcome };

export type TerritoryApi = {
  /** `GET /repository_areas`. A Fleet that has no such route answers none rather than a refusal. */
  getRepositoryAreas: () => Promise<RepositoryAreasRead>;
  /** `GET /slot_touches`. A Fleet that has no such route answers none rather than a refusal. */
  getSlotTouches: () => Promise<SlotTouchesRead>;
};

export const TERRITORY_CHANNELS = {
  getRepositoryAreas: "bridge:get-repository-areas",
  getSlotTouches: "bridge:get-slot-touches",
} as const;
