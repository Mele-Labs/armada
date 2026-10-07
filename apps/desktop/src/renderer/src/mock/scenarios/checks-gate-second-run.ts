import { holding } from "../holding";
import type { Scenario } from "../moment";
import { featureGateSecondRun } from "@armada/jobs/fake";

export const s181GateSecondRun: Scenario = holding("checks/gate-second-run", "A group's gate on its second run, the first run red", [featureGateSecondRun()], {
  opens: featureGateSecondRun().job.id,
});
