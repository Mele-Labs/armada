// Core's members: the connection, the rail's pick, servers, and the acts no surface owns.

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
    return {
      state: async () => fleet.state(),
      // The mock has no main to diff: every change is the whole state.
      subscribeChanges: (onChange) => fleet.listen((state) => onChange({ full: state, seq: 0 })),
      syncState: async () => ({ full: fleet.state(), seq: 0 }),
      // The stand-in walk window sets the dim itself — `walk-window.tsx`.
      onWalkFocus: () => () => {},
      // The rail's pick is this window's own, so it moves here as it does in main.
      pickRepository: async (root) => fleet.publish({ repository: root }),
      startServer: async () => OK,
      stopServer: async () => OK,
      openServerLink: async () => ({ ok: false, why: "no_address" }),
      openLink: async () => ({ ok: true }),
      // The three mismatch scenarios differ only in how this goes: a Fleet launchd holds that comes
      // back matching, one it holds that comes back still apart, and one it does not hold.
      // The Fleet panel's own scenarios restart through `mock/fleet-build.tsx`; the fixtures hold no Fleet to restart.
      changeFleetBuild: async () => OK,
      restartFleet: async () => {
        const now = fleet.state().connection;
        if (scenario.name === "fleet/protocol-mismatch-unmanaged" || !("fleet" in now)) {
          return { ok: false, why: "not_started_by_armada", detail: "gui/501/com.armada.fleet is not loaded" };
        }
        await new Promise((resolve) => setTimeout(resolve, 1500));
        const came = { ...now.fleet, pid: now.fleet.pid + 1 };
        fleet.publish({
          connection:
            scenario.name === "fleet/protocol-mismatch-still-apart" && now.state === "protocol_mismatch"
              ? { ...now, fleet: came }
              : { state: "connected", fleet: came, cursor: 0 },
        });
        return { ok: true };
      },
      onHistory: () => () => undefined,
      // Nothing in the mock presses a notification.
      onSummoned: () => () => undefined,
      // Nothing in the mock opens a link from outside.
      onPullLink: () => () => undefined,
      // The mock provides no haptics, so nothing calls this; answered for the type.
      tap: () => undefined,
      dockCount: () => undefined,
    };
  },
};
