// The Fleet panel's build section, standing in for a Fleet that serves one: where the running build
// stands against main, which build it runs on, and the restart a choice or the button comes to.
// Seeded by scenario name; every other scenario draws no section.

import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import type { FleetBuild, FleetBuildChoice, FleetBuildDrone, FleetBuildPosition } from "@armada/components";

import { FleetBuildFrom } from "../fleet-build";

type Seed = {
  on: FleetBuildChoice;
  position: FleetBuildPosition;
  working: FleetBuildChoice | null;
  drones?: readonly FleetBuildDrone[];
  failed?: string;
};

/** Commits in the preview that main lacks: the in-flight branches, merged. */
const PREVIEW_AHEAD = 4;
/** A restart takes this long to show a Fleet on the other build. */
const RESTART_MS = 2_000;

const WORKING: readonly FleetBuildDrone[] = [
  { id: "01K", label: "Reword the empty states" },
  { id: "01L", label: "Count drones from the roster" },
];

const SEEDS: Record<string, Seed> = {
  "fleet/build-aligned": { on: "main", position: { ahead: 0, behind: 0 }, working: null },
  "fleet/build-behind": { on: "main", position: { ahead: 0, behind: 3 }, working: null },
  "fleet/build-preview-ahead": { on: "preview", position: { ahead: PREVIEW_AHEAD, behind: 0 }, working: null },
  // A frame to look at: its restart is not timed, so it does not finish on its own.
  "fleet/build-refreshing": { on: "preview", position: { ahead: PREVIEW_AHEAD, behind: 0 }, working: "preview" },
  "fleet/build-drones-working": { on: "main", position: { ahead: 0, behind: 3 }, working: null, drones: WORKING },
  "fleet/build-failed": {
    on: "main",
    position: { ahead: 0, behind: 3 },
    working: null,
    failed: "main cannot fast-forward to origin/main — it has diverged or a local change is in the way",
  },
};

const levelWith = (on: FleetBuildChoice): FleetBuildPosition =>
  on === "main" ? { ahead: 0, behind: 0 } : { ahead: PREVIEW_AHEAD, behind: 0 };

function Seeded({ seed, children }: { seed: Seed; children: ReactNode }) {
  const [now, setNow] = useState(seed);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  useEffect(() => () => clearTimeout(timer.current), []);
  const restart = (onto: FleetBuildChoice) => {
    setNow(({ failed: _gone, ...was }) => ({ ...was, working: onto }));
    timer.current = setTimeout(() => setNow({ on: onto, position: levelWith(onto), working: null }), RESTART_MS);
  };
  const build: FleetBuild = {
    on: now.on,
    position: now.position,
    working: now.working,
    drones: now.drones ?? [],
    ...(now.failed === undefined ? {} : { failed: now.failed }),
    onChoose: (on) => restart(on),
    onRestart: () => restart(now.on),
  };
  return <FleetBuildFrom value={build}>{children}</FleetBuildFrom>;
}

/** The app under this scenario's build, where it has one. */
export function MockFleetBuild({ scenario, children }: { scenario: string; children: ReactNode }) {
  const seed = SEEDS[scenario];
  return seed === undefined ? children : <Seeded seed={seed}>{children}</Seeded>;
}
