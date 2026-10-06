// Studios' members: the Studio reads and writes, kept by one Fleet over the scenario's Studios.

import type { Outcome } from "@armada/protocol";

import type { StudiosApi, StudiosState } from "../../../../shared/api/studios";
import { STUDIOS_NOTHING_YET } from "../../../../shared/api/studios";
import type { Slice } from "../fake-context";
import { keeping, studying } from "../studio-fleet";
import { zoning } from "../studio-read-in";
import { zoneProposing } from "../studio-zone-proposal";
import { readingNothing } from "../studio-read-nothing";

export const studios: Slice<StudiosApi, StudiosState> = {
  name: "studios",
  state: STUDIOS_NOTHING_YET,
  scenarios: () => [studying().scenario, zoning().scenario, zoneProposing().scenario, readingNothing().scenario],
  api: (scenario, fleet) => ({
    // Every scenario keeps Studios, so the surface opens wherever it is reached. A scenario naming
    // none keeps an empty list and draws its empty state, never a read failure — #1341.
    ...keeping(scenario.studios).routes({ state: fleet.state, publish: fleet.publish }),
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
      scroll: () => {},
    },
    // A Studio starting one entry — #1289, #1345. The mock Fleet answers, so
    // what these do here is what every other unstubbed act does: nothing.
    startStudioRun: async () => ({ ok: true }) as Outcome,
    startStudioServer: async () => ({ ok: true }) as Outcome,
  }),
};
