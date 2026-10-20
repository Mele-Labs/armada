// The flight controls Kit holds and each repository's, kept in memory for the life of the window, and a shell line that ends the way
// the fake runner's does: `vitest` fails, every other line exits clean. Every repository's are kept On this Mac. A scenario that wants a
// file that will not read, another place, or a line that times out, supplies its own through `Scenario.behaves`.

import type { FlightControls } from "@armada/protocol";

import type { FlightControlsApi } from "../../../../shared/api/flight-controls";
import type { Slice } from "../fake-context";
import { RUN_DEFAULT, RUN_OUTPUT } from "../sessions/script";

export const flightControls: Slice<FlightControlsApi, Record<string, never>> = {
  name: "flightControls",
  state: {},
  api: () => {
    let held: FlightControls = { version: 1, job: [], deck: [] };
    const repositories = new Map<string, FlightControls>();
    const answer = (repository: string) => ({ ok: true as const, value: { repository, place: "this_mac" as const, read: { what: "read" as const, controls: structuredClone(repositories.get(repository) ?? { version: 1, job: [], deck: [] }) }, in_repository: { what: "read" as const, controls: { version: 1, job: [], deck: [] } } } });
    return {
      getFlightControls: async () => ({ ok: true, value: { what: "read", controls: structuredClone(held) } }),
      saveFlightControls: async (controls) => {
        held = structuredClone(controls);
        return { ok: true, value: { what: "read", controls: structuredClone(held) } };
      },
      getRepositoryFlightControls: async (repository) => answer(repository),
      saveRepositoryFlightControls: async (repository, controls) => {
        repositories.set(repository, structuredClone(controls));
        return answer(repository);
      },
      runSessionShell: async (_sessionId, command) => {
        const lines = RUN_OUTPUT.find(([match]) => command.includes(match))?.[1] ?? RUN_DEFAULT;
        return { ok: true, value: { exit: command.includes("vitest") ? 1 : 0, timed_out: false, output: `${lines.join("\n")}\n`, truncated: false } };
      },
    };
  },
};
