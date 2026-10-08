// Fleet's notice above the surface, and the one act that mends a mismatch.
//
// **Restart Fleet is only offered where the connection is a protocol
// mismatch.** The act is `launchctl` in main (`apps/desktop/src/main/
// restart-fleet.ts`) and restarts Fleet onto the `armada` that is installed.
// Bridge cannot build or install one, so a restart that brings back a Fleet
// still apart from this Bridge is a result and is said: the window then has one
// thing to name, which is the build.

import { useEffect, useState } from "react";
import { LoaderCircle, RotateCw } from "lucide-react";
import { Alert, Button, Tooltip } from "@armada/components";
import type { BridgeIdentity, FleetRestart } from "@armada/protocol";

import { FailureBlock } from "./FailureSurface";
import { fleetRestartFailure } from "./failures";
import type { Failure } from "./failures";

/** How long a restart may take to show a new Fleet before it is called unanswered. */
export const RESTART_WAIT_MS = 30_000;

/** What the button costs a working Drone, in the words a person weighs it by. */
const RESTART_TOOLTIP = "Restart Fleet. Working Drones are adopted and cannot be redirected until they finish.";

type Phase =
  | { at: "idle" }
  | { at: "working"; pid: number | null }
  | { at: "failed"; restart: Extract<FleetRestart, { ok: false }> };

export type FleetNoticeProps = {
  fleet: Failure;
  /** The pid the runtime file names, where the connection names a Fleet. */
  fleetPid: number | null;
  bridge: BridgeIdentity;
  onCopied: (value: string) => void;
  restart: () => Promise<FleetRestart>;
};

export function FleetNotice({ fleet, fleetPid, bridge, onCopied, restart }: FleetNoticeProps) {
  const [phase, setPhase] = useState<Phase>({ at: "idle" });
  const mismatch = fleet.payload.code === "bridge.fleet.protocol_mismatch";
  const working = phase.at === "working";
  // A new pid under a notice that is still here: Fleet came back, and it still does not match.
  const apart = working && fleetPid !== null && phase.pid !== null && fleetPid !== phase.pid;

  useEffect(() => {
    if (!working || apart) return;
    const timer = window.setTimeout(
      () => setPhase({ at: "failed", restart: { ok: false, why: "no_answer", detail: `no new runtime file in ${RESTART_WAIT_MS / 1000}s` } }),
      RESTART_WAIT_MS,
    );
    return () => window.clearTimeout(timer);
  }, [working, apart]);

  const press = () => {
    setPhase({ at: "working", pid: fleetPid });
    void restart().then((answer) => {
      if (!answer.ok) setPhase({ at: "failed", restart: answer });
    });
  };

  const button = !mismatch ? null : (
    <Tooltip label={RESTART_TOOLTIP}>
      <Button variant="ghost" size="sm" ground="sunken" pending={working && !apart} disabled={working} onClick={press}>
        {working && !apart ? (
          <LoaderCircle size={16} strokeWidth={2} aria-hidden="true" />
        ) : (
          <RotateCw size={16} strokeWidth={2} aria-hidden="true" />
        )}
        {working && !apart ? "Restarting Fleet" : "Restart Fleet"}
      </Button>
    </Tooltip>
  );

  return (
    <>
      <FailureBlock failure={fleet} onCopied={onCopied} acts={button} />
      {apart ? (
        <Alert tone="escalated" title="Still not matching">
          Fleet restarted on the installed build, and it is not this Bridge's. Update Armada, then reopen Bridge.
        </Alert>
      ) : null}
      {phase.at === "failed" ? (
        <FailureBlock
          failure={fleetRestartFailure(phase.restart, bridge)}
          onCopied={onCopied}
          reloadable={false}
          onDismiss={() => setPhase({ at: "idle" })}
        />
      ) : null}
    </>
  );
}
