// The Job "Header Font Sizes" with its last question open in Needs you, for the walk `band-one-bar`: the question's options are the quick bar's first chips and the band keeps its words. Over `sessions`.

import type { Scenario } from "../moment";
import { HEADER_SIZES_ROWS } from "../real-threads/header-sizes";
import { realThreadScenario } from "../real-threads/real-thread";

const QUESTION = "Do you want the plan recorded now, or keep discussing?";

export const s342BandOneBar: Scenario = realThreadScenario({
  name: "band-one-bar",
  says: "the question's options are the quick bar's first chips and the band keeps its words.",
  id: "1923754a-fbbb-4397-9a72-d41a841e7be1",
  title: "Header Font Sizes",
  branch: "armada/session-1923754a",
  slot: 3,
  rows: HEADER_SIZES_ROWS,
  asked: [],
  withBand: "one-bar",
  ask: {
    command: QUESTION,
    call: "band-one-bar-q1",
    offers: ["allow_once", "refuse"],
    questions: [
      {
        question: QUESTION,
        header: "Plan",
        multi_select: false,
        options: [
          { label: "Record the plan", description: "" },
          { label: "Keep discussing", description: "" },
          { label: "Show size B", description: "" },
        ],
      },
    ],
  },
});
