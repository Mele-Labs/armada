// A mock Fleet whose Drone is held on a command: answering it clears the wait, as Fleet does by
// publishing the job without `command_waiting`. `held/command`, and the walks that play it.

import { arcWatched } from "@armada/screens/src/fixtures/build/arc-base";

import type { BridgeApi } from "../../../shared/api";
import type { FleetHandle } from "./moment";

export function answeringTheHeldCommand(fleet: FleetHandle): Partial<BridgeApi> {
  return {
    answerCommand: async () => {
      const { watched } = fleet.state();
      if (watched.state === "read") {
        fleet.publish({ watched: arcWatched({ ...watched.detail, command_waiting: undefined }) });
      }
      return { ok: true };
    },
  };
}
