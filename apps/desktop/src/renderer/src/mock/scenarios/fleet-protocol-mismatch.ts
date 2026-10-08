// A Fleet whose protocol is not this Bridge's: the runtime file names a live
// pid, Bridge refuses to open a socket. Beside `fleet/starting`, where the same
// pid has simply not answered yet. The walk `fleetProtocolMismatch` plays it.

import { NOTHING_YET } from "../../../../shared/bridge";
import type { Scenario } from "../moment";

export const s081FleetProtocolMismatch: Scenario = {
  name: "fleet/protocol-mismatch",
  says: "Fleet runs another protocol than this Bridge, so Bridge does not connect",
  state: {
    ...NOTHING_YET,
    connection: {
      state: "protocol_mismatch",
      fleet: {
        protocolId: "3fa9c1d20b7e4a15",
        pid: 61372,
        port: 40000,
        startedAt: new Date(Date.now() - 3_600_000).toString(),
      },
      speaks: "3fa9c1d20b7e4a15",
      expected: "91bb07e4c25d8830",
    },
  },
  reads: {},
};
