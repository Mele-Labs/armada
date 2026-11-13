// The notice over a Fleet that is not answering, in the three states that each offer a different act.
// `fleet/repair` opens on a Fleet on an older kept build than the one `builds/current` names (Restart Fleet),
// and its walk `later` step moves on to this Bridge on an older one (Reopen Armada). `slices/core.ts` answers
// `keptBuilds` from the Fleet's build the connection names. The walk `fleetRepair` plays it.
//
// `fleet/not-running` is no Fleet running (Start Fleet). **It has no walk**: the Cockpit holds a Fleet that is
// down as powering on for `POWER_GRACE_MS` (`cockpit/boot.ts`), and the notice draws once that has passed,
// which is longer than a walk waits for anything.

import type { Connection, FleetIdentity } from "@armada/protocol";

import { NOTHING_YET } from "../../../../shared/bridge";
import type { Scenario } from "../moment";

const FLEET: FleetIdentity = {
  protocolId: "3fa9c1d20b7e4a15",
  pid: 61372,
  port: 40000,
  startedAt: new Date(Date.now() - 3_600_000).toString(),
};

/** Fleet on `build`, speaking a protocol this Bridge does not. */
const apart = (build: string): Connection => ({
  state: "protocol_mismatch",
  fleet: {
    ...FLEET,
    build,
  },
  speaks: "3fa9c1d20b7e4a15",
  expected: "91bb07e4c25d8830",
});

export const s084FleetRepair: Scenario = {
  name: "fleet/repair",
  says: "Fleet is on an older kept build than the current one; then this Bridge is",
  state: {
    ...NOTHING_YET,
    connection: apart("2026-11-12T211515Z-2f88990"),
  },
  later: [
    {
      connection: apart("2026-11-13T004852Z-126b4bb"),
    },
  ],
};

export const s085FleetNotRunning: Scenario = {
  name: "fleet/not-running",
  says: "No Fleet is running, and after the Cockpit's wait the notice offers to start the one launchd holds",
  state: {
    ...NOTHING_YET,
    connection: {
      state: "not_running",
      absence: {
        why: "no_runtime_file",
        path: "/Users/user/Library/Application Support/Armada/fleet.json",
      },
    },
  },
};
