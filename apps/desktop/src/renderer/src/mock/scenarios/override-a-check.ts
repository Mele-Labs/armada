// Out of retries on a failed `diff_nonempty`, with the override offered
// beside the two re-runs (Job 12, 6 Oct 2026).

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { awaitingRepairOverridable } from "@armada/jobs/fixtures/build/waiting";

export const s161OverrideACheck: Scenario = holding("repair/override-a-check", awaitingRepairOverridable().name, [awaitingRepairOverridable()], {
  opens: awaitingRepairOverridable().job.id,
});
