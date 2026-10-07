import { holding } from "../holding";
import type { Scenario } from "../moment";
import { featureGateFirstRun } from "@armada/jobs/fake";

export const s180GateFirstRun: Scenario = holding("checks/gate-first-run", "A group's gate on its first run, two Checks skipped", [featureGateFirstRun()], {
  opens: featureGateFirstRun().job.id,
});
