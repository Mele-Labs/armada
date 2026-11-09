// The other take of `plan-as-progress`: the plan's stretches as a list at the head of the story, each with its gate, in place of each gate on the spine under its stretch.

import type { Scenario } from "../moment";
import { planAsProgress } from "./plan-as-progress";

export const s349PlanAsProgressList: Scenario = planAsProgress(
  "plan-as-progress-list",
  "A Job whose plan is its workflow, its stretches listed at the head of the story",
  "list",
);
