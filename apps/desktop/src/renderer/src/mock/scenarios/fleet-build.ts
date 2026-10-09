// The Fleet panel's build section in the four states it takes: aligned with main, behind it, a
// preview ahead of it, and a preview refresh under way. The seeds are `../fleet-build.tsx`'s, by
// name. The walks `fleetBuildAligned`, `fleetBuildBehind`, `fleetBuildPreviewAhead` and
// `fleetBuildRefreshing` play them.

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
