// The Fleet panel's build section, standing in for a Fleet that does not serve one yet: where the
// running build stands against main, which build it runs on, and the restart a choice or a
// refresh comes to. Seeded by scenario name; every other scenario draws no section.

import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import type { FleetBuild, FleetBuildChoice, FleetBuildPosition, FleetBuildWork } from "@armada/components";

import { FleetBuildFrom } from "../fleet-build";

type Seed = { on: FleetBuildChoice; position: FleetBuildPosition; working: FleetBuildWork | null };

/** Commits in the preview that main lacks: the in-flight branches, merged. */
const PREVIEW_AHEAD = 4;
/** A restart takes this long to show a Fleet on the other build. */
const RESTART_MS = 2_000;

const SEEDS: Record<string, Seed> = {
  "fleet/build-aligned": { on: "main", position: { ahead: 0, behind: 0 }, working: null },
  "fleet/build-behind": { on: "main", position: { ahead: 0, behind: 3 }, working: null },
  "fleet/build-preview-ahead": { on: "preview", position: { ahead: PREVIEW_AHEAD, behind: 0 }, working: null },
  // A frame to look at: its restart is not timed, so it does not finish on its own.
  "fleet/build-refreshing": { on: "preview", position: { ahead: PREVIEW_AHEAD, behind: 0 }, working: "refreshing" },
};

function Seeded({ seed, children }: { seed: Seed; children: ReactNode }) {
  const [now, setNow] = useState(seed);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  useEffect(() => () => clearTimeout(timer.current), []);
  const restart = (work: FleetBuildWork, then: Seed) => {
    setNow((was) => ({ ...was, working: work }));
    timer.current = setTimeout(() => setNow(then), RESTART_MS);
  };
  const build: FleetBuild = {
    ...now,
    onChoose: (on) =>
      restart("switching", { on, position: on === "main" ? { ahead: 0, behind: 0 } : { ahead: PREVIEW_AHEAD, behind: 0 }, working: null }),
    onRefresh: () => restart("refreshing", { on: "preview", position: { ahead: now.position.ahead + 1, behind: 0 }, working: null }),
  };
  return <FleetBuildFrom value={build}>{children}</FleetBuildFrom>;
}

/** The app under this scenario's build, where it has one. */
export function MockFleetBuild({ scenario, children }: { scenario: string; children: ReactNode }) {
  const seed = SEEDS[scenario];
  return seed === undefined ? children : <Seeded seed={seed}>{children}</Seeded>;
}
