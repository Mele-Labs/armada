// A Fleet whose runtime file names a pid that is held and which has not
// answered yet, beside `fleet-not-running`, where nothing is held. The walk
// `fleetStarting` plays it.

import { NOTHING_YET } from "../../../../shared/bridge";
import type { Scenario } from "../moment";

export const s080FleetStarting: Scenario = {
  name: "fleet/starting",
  says: "Fleet is starting: the runtime file names a live pid and it has not answered",
  state: {
    ...NOTHING_YET,
    connection: {
      state: "starting",
      fleet: {
        protocolVersion: { major: 13, minor: 50 },
        pid: 61372,
        port: 40000,
        startedAt: new Date(Date.now() - 4_000).toString(),
      },
    },
  },
  reads: {},
};
