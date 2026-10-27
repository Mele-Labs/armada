// The build Fleet runs on, as the Fleet panel draws it: main or the preview, where it stands
// against main, and the one act. `useFleetBuild` is a window's override, which the mock provides
// from a fixture; a real window builds it from what Fleet reported, `fleetBuildOf`.

import { createContext, useContext } from "react";
import type { FleetBuild, FleetBuildChoice } from "@armada/components";
import type { FleetBuildReport } from "@armada/protocol";

const Held = createContext<FleetBuild | undefined>(undefined);

export const FleetBuildFrom = Held.Provider;

export function useFleetBuild(): FleetBuild | undefined {
  return useContext(Held);
}

/** What Fleet reported, as the panel draws it. `undefined` where Fleet offers no build. */
export function fleetBuildOf(
  report: FleetBuildReport | null,
  change: (build: FleetBuildChoice, adopt: boolean) => void,
): FleetBuild | undefined {
  if (report === null) return undefined;
  return {
    on: report.on,
    ...(report.position === undefined ? {} : { position: report.position }),
    working: report.restarting ?? null,
    ...(report.stage === undefined ? {} : { stage: report.stage }),
    drones: [],
    ...(report.failed === undefined ? {} : { failed: report.failed }),
    ...(report.left_out === undefined ? {} : { leftOut: report.left_out }),
    onChoose: change,
    onRestart: (adopt) => change(report.on, adopt),
  };
}
