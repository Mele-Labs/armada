// A mock Fleet whose Drone is held on a command: answering it clears the wait, as Fleet does by
// publishing the job without `command_waiting`. `held/command`, and the walks that play it.

import { arcWatched } from "../fixtures/build/arc-base";

import type { FleetHandle } from "@armada/bridge-api";

import type { JobsApi, JobsState } from "../api";

/** The state the held-command answer writes, and the member: both inside the app's whole state `S` and API `A`. */
export type HeldCommandState = Pick<JobsState, "watched">;

export function answeringTheHeldCommand<S extends HeldCommandState, A extends Pick<JobsApi, "answerCommand">>(
  whole: FleetHandle<S>,
): Partial<A> {
  // `S` is the app's whole state; this member writes only the fields `HeldCommandState` names.
  const fleet = whole as unknown as FleetHandle<HeldCommandState>;
  const members: Pick<JobsApi, "answerCommand"> = {
    answerCommand: async () => {
      const { watched } = fleet.state();
      if (watched.state === "read") {
        fleet.publish({ watched: arcWatched({ ...watched.detail, command_waiting: undefined }) });
      }
      return { ok: true };
    },
  };
  return members as Partial<A>;
}
