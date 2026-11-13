// Fleet's notice above the surface, and the one act that mends it.
//
// **One button, and `fleet-repair.ts` says which.** Fleet down is Start Fleet. A protocol mismatch is
// Restart Fleet where Fleet is the stale side, and Reopen Armada where this Bridge is. Restart is
// `launchctl` in main (`apps/desktop/src/main/restart-fleet.ts`) and restarts Fleet onto the `armada`
// that is installed. Bridge cannot build or install one, so a restart that brings back a Fleet still
// apart from this Bridge is a result and is said: the window then has one thing to name, which is the
// build. Start and Reopen are `apps/desktop/src/main/start-fleet.ts`.

import { useEffect, useState } from "react";
import { LoaderCircle, RotateCw } from "lucide-react";
import { Alert, Button, Tooltip } from "@armada/components";
import type {
  BridgeIdentity,
  BridgeReopen,
  FleetRestart,
  FleetStart,
  KeptBuilds,
} from "@armada/protocol";

import { FailureBlock } from "./FailureSurface";
import { bridgeReopenFailure, fleetRestartFailure, fleetStartFailure } from "./failures";
import type { Failure } from "./failures";
import { repairOf } from "./fleet-repair";
import type { Repair } from "./fleet-repair";

/** How long a start, restart or reopen may take to show a new Fleet, or a new window, before it is called unanswered. */
export const RESTART_WAIT_MS = 30_000;

/** What each button does, as a fact, and for Restart what it costs a working Drone. */
const TOOLTIPS: Record<Repair, string> = {
  start: "Start Fleet. Bridge reconnects when it answers.",
  restart: "Restart Fleet. Working Drones are adopted and cannot be redirected until they finish.",
  reopen:
    "Reopen Armada on the build Fleet runs. Fleet and its Drones are not touched, and this window closes.",
};

const LABELS: Record<
  Repair,
  {
    idle: string;
    working: string;
  }
> = {
  start: {
    idle: "Start Fleet",
    working: "Starting Fleet",
  },
  restart: {
    idle: "Restart Fleet",
    working: "Restarting Fleet",
  },
  reopen: {
    idle: "Reopen Armada",
    working: "Reopening Armada",
  },
};

type Phase =
  | {
      at: "idle";
    }
  | {
      at: "working";
      repair: Repair;
      pid: number | null;
    }
  | {
      at: "failed";
      failure: Failure;
    };

export type FleetNoticeProps = {
  fleet: Failure;
  /** The pid the runtime file names, where the connection names a Fleet. */
  fleetPid: number | null;
  /** The kept build folder Fleet runs from, where the runtime file names one. */
  fleetBuild: string | null;
  bridge: BridgeIdentity;
  onCopied: (value: string) => void;
  start: () => Promise<FleetStart>;
  restart: () => Promise<FleetRestart>;
  reopen: () => Promise<BridgeReopen>;
  /** The kept builds this Bridge and the install are, read when a mismatch needs telling which side is stale. */
  builds: () => Promise<KeptBuilds>;
};

export function FleetNotice({
  fleet,
  fleetPid,
  fleetBuild,
  bridge,
  onCopied,
  start,
  restart,
  reopen,
  builds,
}: FleetNoticeProps) {
  const [phase, setPhase] = useState<Phase>({
    at: "idle",
  });
  const [read, setRead] = useState<KeptBuilds | null>(null);
  const code = fleet.payload.code;
  const mismatch = code === "bridge.fleet.protocol_mismatch";
  const repair = repairOf(code, fleetBuild, read);
  const working = phase.at === "working";
  // A new pid under a notice that is still here: Fleet came back, and it still does not match.
  const apart =
    working &&
    phase.repair === "restart" &&
    fleetPid !== null &&
    phase.pid !== null &&
    fleetPid !== phase.pid;

  // Which side is stale is read where it can be told: a mismatch, and again when Fleet comes back on another build. A read that fails leaves Restart Fleet, as before.
  useEffect(() => {
    if (!mismatch) return;
    let live = true;
    builds().then(
      (answer) => live && setRead(answer),
      () =>
        live &&
        setRead({
          bridge: null,
          current: null,
        }),
    );
    return () => {
      live = false;
    };
  }, [
    mismatch,
    builds,
    fleetBuild,
  ]);

  useEffect(() => {
    if (!working || apart) return;
    const unanswered = (repair: Repair): Failure =>
      repair === "reopen"
        ? bridgeReopenFailure(
            {
              ok: false,
              detail: `no new window in ${RESTART_WAIT_MS / 1000}s`,
            },
            bridge,
          )
        : repair === "start"
          ? fleetStartFailure(
              {
                ok: false,
                why: "no_answer",
                detail: `no new runtime file in ${RESTART_WAIT_MS / 1000}s`,
              },
              bridge,
            )
          : fleetRestartFailure(
              {
                ok: false,
                why: "no_answer",
                detail: `no new runtime file in ${RESTART_WAIT_MS / 1000}s`,
              },
              bridge,
            );
    const timer = window.setTimeout(
      () =>
        setPhase({
          at: "failed",
          failure: unanswered(phase.repair),
        }),
      RESTART_WAIT_MS,
    );
    return () => window.clearTimeout(timer);
  }, [
    working,
    apart,
    phase,
    bridge,
  ]);

  const press = (act: Repair) => {
    setPhase({
      at: "working",
      repair: act,
      pid: fleetPid,
    });
    const failed = (failure: Failure) =>
      setPhase({
        at: "failed",
        failure,
      });
    if (act === "start")
      void start().then((answer) => {
        if (!answer.ok) failed(fleetStartFailure(answer, bridge));
      });
    else if (act === "reopen")
      void reopen().then((answer) => {
        if (!answer.ok) failed(bridgeReopenFailure(answer, bridge));
      });
    else
      void restart().then((answer) => {
        if (!answer.ok) failed(fleetRestartFailure(answer, bridge));
      });
  };

  const pending = working && !apart;
  const button =
    repair === null ? null : (
      <Tooltip label={TOOLTIPS[repair]}>
        <Button
          variant="ghost"
          size="sm"
          ground="sunken"
          pending={pending}
          disabled={working}
          onClick={() => press(repair)}
        >
          {pending ? (
            <LoaderCircle size={16} strokeWidth={2} aria-hidden="true" />
          ) : (
            <RotateCw size={16} strokeWidth={2} aria-hidden="true" />
          )}
          {pending ? LABELS[repair].working : LABELS[repair].idle}
        </Button>
      </Tooltip>
    );

  return (
    <>
      <FailureBlock failure={fleet} onCopied={onCopied} acts={button} />
      {apart ? (
        <Alert tone="escalated" title="Still not matching">
          Fleet restarted on the installed build, and it is not this Bridge's. Update Armada, then
          reopen Bridge.
        </Alert>
      ) : null}
      {phase.at === "failed" ? (
        <FailureBlock
          failure={phase.failure}
          onCopied={onCopied}
          reloadable={false}
          onDismiss={() =>
            setPhase({
              at: "idle",
            })
          }
        />
      ) : null}
    </>
  );
}
