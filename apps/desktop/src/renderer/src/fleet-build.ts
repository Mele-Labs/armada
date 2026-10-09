// The build Fleet runs on, as the Fleet panel draws it: main or the preview, where it stands
// against main, and the one act. `useFleetBuild` is a window's override, which the mock provides
// from a fixture; a real window builds it from what Fleet reported, `fleetBuildOf`.

import { createContext, useContext } from "react";
import type { FleetBuild, FleetBuildChoice, FleetBuildDrone } from "@armada/components";
import type { FleetBuildReport, JobSummary } from "@armada/protocol";

const Held = createContext<FleetBuild | undefined>(undefined);

export const FleetBuildFrom = Held.Provider;

export function useFleetBuild(): FleetBuild | undefined {
  return useContext(Held);
}

/**
 * The Jobs a restart would adopt: those at `running`, the one status a live Drone stands for.
 * **`scripts/restart` reads the same thing off the roster** and refuses without `--adopt`, so this
 * is what the person is shown before saying yes.
 */
export function dronesWorking(jobs: readonly JobSummary[]): FleetBuildDrone[] {
  return jobs.filter((job) => job.status === "running").map((job) => ({ id: job.id, label: job.title }));
}

/** What Fleet reported, as the panel draws it. `undefined` where Fleet offers no build. */
export function fleetBuildOf(
  report: FleetBuildReport | null,
  jobs: readonly JobSummary[],
  change: (build: FleetBuildChoice, adopt: boolean) => void,
): FleetBuild | undefined {
  if (report === null) return undefined;
  return {
    on: report.on,
    ...(report.position === undefined ? {} : { position: report.position }),
    working: report.restarting ?? null,
    drones: dronesWorking(jobs),
    ...(report.failed === undefined ? {} : { failed: report.failed }),
    onChoose: change,
    onRestart: (adopt) => change(report.on, adopt),
  };
}
