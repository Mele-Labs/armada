// The flight deck with a Job whose flight controls outgrow two rows, for the walk `flight-controls-overflow`: the thirteen built-in tiles and
// four actions the repository keeps, seventeen in all. Over `flight-controls-custom`'s Job and bay. The walk is played in two takes,
// `?take=wider` and `?take=peek` (`cockpit/deck/flight-take.ts`). The owner's ask of 23 Oct 2026.

import type { FlightControls } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import type { Scenario } from "../moment";
import { flightControlsCustom } from "./flight-controls-custom";

const FOUR: FlightControls = {
  version: 1,
  job: [
    { id: "hand-off", kind: "words", name: "Hand off", text: "Write down what is left, then stop." },
    { id: "review", kind: "command", name: "Review", text: "/review" },
    { id: "summarise", kind: "words", name: "Summarise", text: "Summarise what changed in three lines." },
    { id: "docs", kind: "page", name: "Docs", text: "https://docs.example/armada" },
  ],
  deck: [],
};

export const flightControlsOverflow: Scenario = {
  ...flightControlsCustom,
  name: "flight-controls-overflow",
  says: "A Job with seventeen flight controls: more than two rows hold, and the take decides how the grid shows it",
  draft: { ...flightControlsCustom.draft, repositoryTiles: { [repository().root]: FOUR } },
};
