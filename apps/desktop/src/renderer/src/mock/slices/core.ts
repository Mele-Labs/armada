// Core's members: the connection, the rail's pick, servers, and the acts no surface owns.

import { PROTOCOL_VERSION } from "@armada/protocol";
import type { Outcome } from "@armada/protocol";

import type { CoreApi, CoreState } from "../../../../shared/api/core";
import { CORE_NOTHING_YET } from "../../../../shared/api/core";
import type { BridgeState } from "../../../../shared/bridge";
import type { Slice } from "../fake-context";

const OK: Outcome = { ok: true };

export const core: Slice<CoreApi<BridgeState>, CoreState> = {
  name: "core",
  state: CORE_NOTHING_YET,
  api: (scenario, fleet) => {
    const summoners = new Set<(to: { jobId: string | null }) => void>();
    let summoned = false;
    return {
      protocolVersion: () => PROTOCOL_VERSION,
      state: async () => fleet.state(),
      subscribe: fleet.listen,
      // The stand-in walk window sets the dim itself — `walk-window.tsx`.
      onWalkFocus: () => () => {},
      // The rail's pick is this window's own, so it moves here as it does in main.
      pickRepository: async (root) => fleet.publish({ repository: root }),
      startServer: async () => OK,
      stopServer: async () => OK,
      openServerLink: async () => ({ ok: false, why: "no_address" }),
      openLink: async () => ({ ok: true }),
      // A scenario opens its Job the way a pressed notification does — once, though `StrictMode` registers twice.
      onHistory: () => () => undefined,
      onSummoned: (onGo) => {
        summoners.add(onGo);
        if (scenario.opens !== undefined && !summoned) {
          summoned = true;
          const to = { jobId: scenario.opens };
          queueMicrotask(() => summoners.forEach((summoner) => summoner(to)));
        }
        return () => summoners.delete(onGo);
      },
      // The mock provides no haptics, so nothing calls this; answered for the type.
      tap: () => undefined,
    };
  },
};
