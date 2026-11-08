// The other take of `plan-as-progress`: each subagent at work stands on the group whose paths it edits, drawn as a guess, since no agent names the group it dispatches for.

import type { Scenario } from "../moment";
import { planAsProgress } from "./plan-as-progress";

export const s349PlanAsProgressPlaced: Scenario = planAsProgress(
  "plan-as-progress-placed",
  "A Job whose plan is its workflow, with each subagent on the group whose paths it edits",
  true,
);
