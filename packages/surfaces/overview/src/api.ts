// Overview: Fleet's health and per-repository drift.
// A slice imports protocol and screens, never another slice; desktop's `shared/api.ts` composes them.

import type { DriftsRead, HealthRead } from "@armada/screens/src/overview-reads";

export type OverviewApi = {
  /**
   * Fleet's health and per-repository drift, or `false` to stop. **Reads.** `App.tsx` holds this
   * open for the life of the window: Overview's own tiles and the left column's Stats and Fleet
   * panels (Bridge/1088) all draw it.
   */
  watchOverview: (want: boolean) => Promise<void>;
};

export type OverviewState = {
  /**
   * `GET /health` — what Fleet can say of its own health, and what it did not probe. Overview's
   * Doctor tile, and the left column's Fleet panel. **Held open for the life of the window**
   * since Bridge/1088: the Fleet panel draws it on every surface, not only Overview's.
   *
   * **This window's own** — `PickedView`.
   */
  health: HealthRead;
  /**
   * Drift for every repository in the scope — each served on All, or the one picked — beside
   * `manifestDrift`, which is the Manifest surface's one. Overview's drift tile, and the left
   * column's Manifest row. Held open for the life of the window, `health`'s reason.
   *
   * **This window's own**, `health`'s reason: the scope is this window's own pick.
   */
  drifts: DriftsRead;
};

export const OVERVIEW_NOTHING_YET: OverviewState = {
  health: { state: "none" },
  drifts: { state: "none" },
};

export const OVERVIEW_CHANNELS = {
  // Overview's health and per-repository drift: one read the surface holds open.
  watchOverview: "bridge:watch-overview",
} as const;
