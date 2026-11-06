// The build Fleet runs on, as the Fleet panel draws it: main or the preview, where it stands
// against main, and the one act. `useFleetBuild` is a window's override, which the mock provides
// from a fixture; a real window builds it from what Fleet reported, `fleetBuildOf`.

import { createContext, useContext } from "react";
import type { FleetBuild, FleetBuildChoice, FleetBuildKept } from "@armada/components";
import type { FleetBuildReport, KeptBuildRunning } from "@armada/protocol";

const Held = createContext<FleetBuild | undefined>(undefined);

export const FleetBuildFrom = Held.Provider;

export function useFleetBuild(): FleetBuild | undefined {
  return useContext(Held);
}

const MONTHS = [
  "Jan",
  "Feb",
  "Mar",
  "Apr",
  "May",
  "Jun",
  "Jul",
  "Aug",
  "Sep",
  "Oct",
  "Nov",
  "Dec",
];

/** An instant in this window's local time, as a kept build is named: `5 Nov, 17:05`. Text that is no instant comes back as it came. */
export function builtSaid(at: string): string {
  const when = new Date(at);
  if (Number.isNaN(when.getTime())) return at;
  const two = (n: number) => String(n).padStart(2, "0");
  return `${when.getDate()} ${MONTHS[when.getMonth()]}, ${two(when.getHours())}:${two(when.getMinutes())}`;
}

/** A kept build as the panel names it: when it was built, and its short commit. */
function keptOf(kept: KeptBuildRunning): FleetBuildKept {
  return {
    built: builtSaid(kept.built_at),
    commit: kept.commit.slice(0, 7),
  };
}

/** What Fleet reported, as the panel draws it. `undefined` where Fleet offers no build. */
export function fleetBuildOf(
  report: FleetBuildReport | null,
  change: (build: FleetBuildChoice, adopt: boolean) => void,
  revert: () => void,
): FleetBuild | undefined {
  if (report === null) return undefined;
  return {
    on: report.on,
    ...(report.position === undefined
      ? {}
      : {
          position: report.position,
        }),
    working: report.restarting ?? null,
    ...(report.stage === undefined
      ? {}
      : {
          stage: report.stage,
        }),
    drones: [],
    ...(report.failed === undefined
      ? {}
      : {
          failed: report.failed,
        }),
    ...(report.left_out === undefined
      ? {}
      : {
          leftOut: report.left_out,
        }),
    ...(report.running === undefined
      ? {}
      : {
          running: keptOf(report.running),
        }),
    ...(report.previous === undefined
      ? {}
      : {
          previous: {
            ...keptOf(report.previous),
            bridgeReopens: report.previous.bridge_reopens ?? false,
            takesOut: report.previous.takes_out ?? [],
            bringsBack: report.previous.brings_back ?? [],
          },
        }),
    ...(report.reverting === true
      ? {
          reverting: true,
        }
      : {}),
    ...(report.carried === undefined
      ? {}
      : {
          carried: report.carried,
        }),
    onChoose: change,
    onRestart: (adopt) => change(report.on, adopt),
    onRevert: revert,
  };
}
