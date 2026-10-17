// The standing conditions, above whatever surface is open.
//
// **A band, not a screen.** Everything here is true regardless of which surface
// a person is on, stays on screen until they put it away, and leaves the
// surface working beneath it — which is the design contract's own definition of
// a banner, and the reason none of it belongs in the status bar.
//
// **A press that failed or was not sent is not a standing condition, so it is
// not here.** It pops up as a toast over everything — `raised.tsx`.
//
// # Why it is its own file
//
// `App.tsx` reached the 900 lines the gate refuses. This band is the one seam
// inside it that is already a seam: `App` decides what is true and this decides
// how a standing truth is drawn, so the props below are exactly the conditions
// and nothing about the surface underneath. The same remedy `protocol.ts` has
// taken three times.
//
// **Nothing here dismisses on a timer.** A person who has just saved a file, or
// gone to look at a worktree, is not looking at this window; a notice that
// expired while they were is one they cannot get back.

import type { ReactNode } from "react";
import { Alert, Button, ManifestNotice } from "@armada/components";
import type { BridgeIdentity, Connection, FleetRestart, ManifestReading } from "@armada/protocol";
import type { Failure } from "@armada/shell";
import { FleetNotice } from "@armada/shell";

export type StandingProps = {
  /** Fleet, where the one connection is not one. */
  fleet: Failure | null;
  /** Where the runtime file's pid is read from, which a restart that did not take shows by moving. */
  connection: Connection;
  bridge: BridgeIdentity;
  /** Restart Fleet, offered where the connection is a protocol mismatch. */
  onRestartFleet: () => Promise<FleetRestart>;
  /** What Fleet's last read of `armada.yml` came to. */
  manifestReading: ManifestReading | null;
  /** The reading already put away, by the instant Fleet read the file. */
  readingSeen: string | null;
  onReadingSeen: (at: string | null) => void;
  /** A clipboard write is silent, so the surface confirms it. */
  onCopied: (value: string) => void;
  /** Events Fleet dropped, and how many of them have been read. */
  missed: number;
  acknowledged: number;
  onAcknowledged: (missed: number) => void;
  /** A clone that finished after its dialog closed — `LocatedNotice`, held by Locate. */
  located?: ReactNode;
};

/** Everything true above the surface, in the order it is met. */
export function Standing({
  fleet,
  connection,
  bridge,
  onRestartFleet,
  manifestReading,
  readingSeen,
  onReadingSeen,
  onCopied,
  missed,
  acknowledged,
  onAcknowledged,
  located,
}: StandingProps) {
  return (
    <>
      {/* Fleet, when the one connection is not one. The status bar keeps the
          single line; this is the same reading with the four runtime-file
          answers and the log under it. */}
      {fleet === null ? null : (
        <FleetNotice fleet={fleet} fleetPid={"fleet" in connection ? connection.fleet.pid : null} bridge={bridge} onCopied={onCopied} restart={onRestartFleet} />
      )}

      {/* Fleet's own Manifest read. **Here and not in the status bar**, which
          the contract holds to three states and one colour and warns must not
          become a second alert surface. Put away by hand: whoever saved the
          file is looking at their editor. Nothing draws for a read with no
          news in it — `ManifestNotice` asks that itself. */}
      {manifestReading === null || readingSeen === manifestReading.at ? null : (
        <ManifestNotice
          reading={manifestReading}
          onDismiss={() => onReadingSeen(manifestReading.at)}
        />
      )}

      {missed <= acknowledged ? null : (
        <Alert
          tone="escalated"
          title="Events were dropped before Bridge saw them"
          action={
            <Button variant="ghost" size="sm" onClick={() => onAcknowledged(missed)}>
              Noted
            </Button>
          }
        >
          {`${missed} events will never arrive. Fleet resynced current state after each drop, so the list below is repaired.`}
        </Alert>
      )}

      {located}
    </>
  );
}
