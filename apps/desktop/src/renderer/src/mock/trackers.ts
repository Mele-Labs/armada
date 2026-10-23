// Mock-only: the tracker mods on a machine in the `mods-trackers` scenario, as `list_mods` would answer them.
// `list_mods` leaves `tried` off a mod whose `tracker.toml` was edited after its try, exactly as it does for a mod
// never tried, so `STALE` is the mock saying which of the two it is. Installed once the window has reset the
// sources, which would otherwise clear them.

import { parseLayout } from "@armada/shell";
import { createTrackerSource, trackerFromWire } from "@armada/settings";
import type { ModSummary } from "@armada/protocol";

import { mockLayout, TIDY_DASHBOARD } from "./layout";
import { DUSK_MOD, mockThemes } from "./themes";

export const mockTrackers = createTrackerSource();

const ON = { kind: "tracker", version: "0.1.0", enabled: true, valid: true } as const;

const WIRE: readonly ModSummary[] = [
  // Tried as it stands, valid, on: a Manifest naming `linear` uses it.
  { ...ON, name: "linear", tried: { at: "2026-10-20T09:14:00.000Z", manifest_id: "armada", passed: true, listed: 12, read: "ARM-41" } },
  // Valid and on, edited since its try.
  { ...ON, name: "shortcut" },
  // Refused: `mod.toml` is fine, `tracker.toml` is not.
  { ...ON, name: "plane", valid: false, reason: "tracker.toml: [read] has no argv, and 1 more" },
];
const STALE = new Set(["shortcut"]);

export function installTrackerMods(): void {
  mockThemes.install(DUSK_MOD);
  mockLayout.install({ name: "tidy", title: "Tidy", enabled: false, file: parseLayout(TIDY_DASHBOARD).file });
  for (const row of WIRE) {
    const mod = trackerFromWire(row, STALE.has(row.name));
    if (mod !== undefined) mockTrackers.install(mod);
  }
}
