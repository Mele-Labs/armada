// Mods with trackers beside themes and layouts: one list, a tracker tried and on, one stale, one refused.
// The walk `mods-trackers` plays it. The mods are installed by `mount.tsx` (`../trackers.ts`).

import { manifesting } from "../manifest-fake";
import type { Scenario } from "../moment";

function build(): Scenario {
  return {
    ...manifesting({}),
    name: "mods-trackers",
    says: "Mods: themes, layouts and trackers in one list, with each tracker tried, valid and switched on",
  };
}

export const modsTrackers: Scenario = build();
