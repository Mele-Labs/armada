// The Judge did not answer on the plan: both recourses, ask again and accept (Job 3, 5 Oct 2026).

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { featureUndecided } from "@armada/jobs/fake";

export const s150JudgeUndecided: Scenario = holding("judge/undecided", "The Judge did not answer, the step stopped", [featureUndecided()], {
  opens: featureUndecided().job.id,
});
