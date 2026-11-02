// The Job "Header Font Sizes" with its last question open in Needs you, for the walk `band-replaces-bar`: the quick bar is not drawn while the band is up. Over `sessions`.

import type { Scenario } from "../moment";
import { HEADER_SIZES_ROWS } from "../real-threads/header-sizes";
import { realThreadScenario } from "../real-threads/real-thread";

const QUESTION = "Do you want the plan recorded now, or keep discussing?";

export const s342BandReplacesBar: Scenario = realThreadScenario({
  name: "band-replaces-bar",
  says: "the quick bar is not drawn while the band is up.",
  id: "1923754a-fbbb-4397-9a72-d41a841e7be1",
  title: "Header Font Sizes",
  branch: "armada/session-1923754a",
  slot: 3,
  rows: HEADER_SIZES_ROWS,
  asked: [],
  ask: {
    command: QUESTION,
    call: "band-replaces-bar-q1",
    offers: [
      "allow_once",
      "refuse",
    ],
    questions: [
      {
        question: QUESTION,
        header: "Plan",
        multi_select: false,
        options: [
          {
            label: "Record the plan",
            description: "",
          },
          {
            label: "Keep discussing",
            description: "",
          },
          {
            label: "Show size B",
            description: "",
          },
        ],
      },
    ],
  },
});
