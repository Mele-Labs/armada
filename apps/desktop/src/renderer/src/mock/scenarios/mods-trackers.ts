// Tracker mods in the Mods list beside a theme and a layout: one tried and on, one edited since its try, one refused. The tracker mods
// are `list_mods` rows in the state, as Fleet answers them. The walk `mods-trackers` plays it; `mount.tsx` installs the theme and the
// layout (`../trackers.ts`).

import type { ModSummary } from "@armada/protocol";

import { manifesting } from "../manifest-fake";
import type { Scenario } from "../moment";

const ON = { kind: "tracker", version: "0.1.0", enabled: true, valid: true } as const;

const MODS: readonly ModSummary[] = [
  // Tried as it stands, valid, on: a Manifest naming `linear` uses it.
  { ...ON, name: "linear", description: "Linear issues", changed_at: "2026-10-19T16:02:00.000Z", tried: { at: "2026-10-20T09:14:00.000Z", manifest_id: "armada", passed: true, listed: 12, read: "ARM-41" } },
  // Valid and on, with no try on the wire: the file was edited after its try, which `trackers.ts` says.
  { ...ON, name: "shortcut", description: "Shortcut stories", changed_at: "2026-10-22T11:40:00.000Z" },
  // Refused: `mod.toml` reads, `tracker.toml` does not pass.
  { ...ON, name: "plane", description: "Plane work items", changed_at: "2026-10-21T08:30:00.000Z", valid: false, reason: "tracker.toml: [read] has no argv, and 1 more" },
];

function build(): Scenario {
  const base = manifesting({});
  return {
    ...base,
    name: "mods-trackers",
    says: "Mods: a theme, a layout and tracker mods in one list, each tracker tried, valid and switched on",
    state: { ...base.state, mods: { mods: [...MODS] } },
  };
}

export const modsTrackers: Scenario = build();
