// The Fleet panel's build section in the states it takes: aligned with main, behind it, a preview
// ahead of it, a restart under way, a Drone working under a restart, and a restart that failed. The
// seeds are `../fleet-build.tsx`'s, by name. The walks `fleetBuildAligned`, `fleetBuildBehind`,
// `fleetBuildPreviewAhead`, `fleetBuildRefreshing`, `fleetBuildDronesWorking` and `fleetBuildFailed`
// play them.

import { repository, workflow } from "@armada/screens/src/fixtures/build/base";

import { connected } from "../moment";
import type { Scenario } from "../moment";

const quiet = (name: string, says: string): Scenario => ({
  name,
  says,
  state: connected([], [workflow()], [repository()]),
  reads: {},
});

export const s084FleetBuildAligned = quiet("fleet/build-aligned", "Fleet runs on main, level with it");
export const s085FleetBuildBehind = quiet("fleet/build-behind", "Fleet runs on main, three commits behind it");
export const s086FleetBuildPreviewAhead = quiet("fleet/build-preview-ahead", "Fleet runs on the preview, four commits ahead of main");
export const s087FleetBuildRefreshing = quiet("fleet/build-refreshing", "The preview is being refreshed and Fleet restarts on it");
export const s088FleetBuildDronesWorking = quiet("fleet/build-drones-working", "Fleet is behind main with two Drones working");
export const s089FleetBuildFailed = quiet("fleet/build-failed", "The last restart did not take");
