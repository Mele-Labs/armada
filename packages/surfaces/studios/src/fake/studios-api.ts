// Studios' members as a mock Fleet answers them: the Studio reads and writes, kept over the
// scenario's Studios, and the few acts the mock has no window for.

import type { FleetHandle } from "@armada/bridge-api";
import type { Outcome, Studio } from "@armada/protocol";

import type { StudiosApi } from "../api";
import { keeping } from "./studio-fleet";
import type { StudioHeld } from "./studio-fleet";

export function studiosApi(seeded: readonly Studio[] | undefined, fleet: FleetHandle<StudioHeld>): StudiosApi {
  return {
    // Every scenario keeps Studios, so the surface opens wherever it is reached. A scenario naming
    // none keeps an empty list and draws its empty state, never a read failure — #1341.
    ...keeping(seeded).routes({ state: fleet.state, publish: fleet.publish }),
    // The capture window is a second window main opens — #1294. The mock has
    // none, so this says what a Studio nothing is holding would say.
    openCaptureWindow: async () => ({ ok: false, why: "no_studio" }),
    captureWindow: {
      read: () => () => {},
      arm: async () => null,
      aim: async () => null,
      hold: async () => null,
      release: async () => {},
      save: async () => ({ ok: true }) as Outcome,
      reload: async () => {},
      followRefused: async () => {},
      approve: async () => ({ ok: true }) as Outcome,
      scroll: () => {},
    },
    // A Studio starting one entry — #1289, #1345. The mock Fleet answers, so
    // what these do here is what every other unstubbed act does: nothing.
    startStudioRun: async () => ({ ok: true }) as Outcome,
    startStudioServer: async () => ({ ok: true }) as Outcome,
  };
}
