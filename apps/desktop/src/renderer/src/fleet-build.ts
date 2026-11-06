// The build Fleet runs on, as the Fleet panel draws it: main or the preview, where it stands
// against main, and the one act. `useFleetBuild` is a window's override, which the mock provides
// from a fixture; a real window builds it from what Fleet reported, `fleetBuildOf`.

import { createContext, useContext, useEffect, useRef } from "react";
import type { FleetBuild, FleetBuildChoice, FleetBuildKept } from "@armada/components";
import type { FleetBuildReport, KeptBuildRunning, Outcome, SnapshotState } from "@armada/protocol";
import { playUiSound } from "./ui-sounds";
import { askToTell, raiseOutcome } from "./tell";

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

/** The presses on the snapshot of picked Jobs, which the panel draws beside the restart's. */
export type SnapshotActs = {
  /** Take a branch out of the picked Jobs. */
  unpick: (branch: string) => void;
  swap: () => void;
};

/**
 * What a pick's refusal says, where the person pressed. **A Job that holds no branch is said on the row** in the
 * words the row uses; any other refusal is the window's toast, with Fleet's own message.
 */
export function pickRefusedSaid(outcome: Outcome): void {
  if (outcome.ok !== false) return;
  if (outcome.why === "refused" && outcome.error.code === "fleet.build_no_branch")
    askToTell("This Job holds no branch yet");
  else raiseOutcome(outcome);
}

/** A press on the build whose refusal has no row to say it on: the window's toast, with Fleet's own message. */
export function settleFleetBuild(outcome: Outcome): void {
  if (outcome.ok === false) raiseOutcome(outcome);
}

/**
 * Plays the chime once when the snapshot turns ready. **Only the move counts**: a report that already
 * holds a ready snapshot when the window opens is not news, and a read that repeats it is not either.
 */
export function useSnapshotChime(report: FleetBuildReport | null): void {
  const was = useRef<SnapshotState | "none" | undefined>(undefined);
  useEffect(() => {
    // Fleet offering no build, or not answering, says nothing about the snapshot.
    if (report === null) return;
    const now = report.snapshot?.state ?? "none";
    if (was.current !== undefined && was.current !== "ready" && now === "ready")
      playUiSound("press");
    was.current = now;
  }, [
    report,
  ]);
}

/** What Fleet reported, as the panel draws it. `undefined` where Fleet offers no build. */
export function fleetBuildOf(
  report: FleetBuildReport | null,
  change: (build: FleetBuildChoice, adopt: boolean) => void,
  revert: () => void,
  snapshot?: SnapshotActs,
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
    ...(report.picked === undefined
      ? {}
      : {
          picked: report.picked,
        }),
    ...(report.snapshot === undefined
      ? {}
      : {
          snapshot: {
            state: report.snapshot.state,
            ...(report.snapshot.stage === undefined
              ? {}
              : {
                  stage: report.snapshot.stage,
                }),
            jobs: report.snapshot.jobs ?? [],
            bridgeReopens: report.snapshot.bridge_reopens ?? false,
            ...(report.snapshot.failed === undefined
              ? {}
              : {
                  failed: report.snapshot.failed,
                }),
          },
        }),
    ...(report.swapping === true
      ? {
          swapping: true,
        }
      : {}),
    ...(snapshot === undefined
      ? {}
      : {
          onUnpick: snapshot.unpick,
          onSwap: snapshot.swap,
        }),
    onChoose: change,
    onRestart: (adopt) => change(report.on, adopt),
    onRevert: revert,
  };
}
