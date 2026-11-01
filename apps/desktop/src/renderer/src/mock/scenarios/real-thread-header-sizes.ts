// The Job "Header Font Sizes" as it really went, in the new thread look, for the walk `real-thread-header-sizes`. Over `sessions`.

import type { Scenario } from "../moment";
import { HEADER_SIZES_ROWS } from "../real-threads/header-sizes";
import { realThreadScenario } from "../real-threads/real-thread";

export const s341cRealThreadHeaderSizes: Scenario = realThreadScenario({
  name: "real-thread-header-sizes",
  says: "A real Job's thread with many commands, his answers to its questions and the agent asking whether to record the plan",
  id: "1923754a-fbbb-4397-9a72-d41a841e7be1",
  title: "Header Font Sizes",
  branch: "armada/session-1923754a",
  slot: 3,
  rows: HEADER_SIZES_ROWS,
  asked: ["Record the plan", "Keep discussing", "Show size B"],
});
