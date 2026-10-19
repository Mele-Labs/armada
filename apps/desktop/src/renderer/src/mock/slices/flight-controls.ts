// The flight controls Kit holds, kept in memory for the life of the window, and a shell line that ends the way the fake runner's
// does: `vitest` fails, every other line exits clean. A scenario that wants a file that will not read, or a line that times out,
// supplies its own through `Scenario.behaves`.

import type { FlightControls } from "@armada/protocol";

import type { FlightControlsApi } from "../../../../shared/api/flight-controls";
import type { Slice } from "../fake-context";
import { RUN_DEFAULT, RUN_OUTPUT } from "../sessions/script";

export const flightControls: Slice<FlightControlsApi, Record<string, never>> = {
  name: "flightControls",
  state: {},
  api: () => {
    let held: FlightControls = { version: 1, job: [], deck: [] };
    return {
      getFlightControls: async () => ({ ok: true, value: { what: "read", controls: structuredClone(held) } }),
      saveFlightControls: async (controls) => {
        held = structuredClone(controls);
        return { ok: true, value: { what: "read", controls: structuredClone(held) } };
      },
      runSessionShell: async (_sessionId, command) => {
        const lines = RUN_OUTPUT.find(([match]) => command.includes(match))?.[1] ?? RUN_DEFAULT;
        return { ok: true, value: { exit: command.includes("vitest") ? 1 : 0, timed_out: false, output: `${lines.join("\n")}\n`, truncated: false } };
      },
    };
  },
};
