// Run Checks again pressed on a Job out of retries: the step still stopped,
// Fleet offering nothing, and its Checks running (Job 3, 4 Oct 2026).

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { awaitingRepairChecksAgain } from "@armada/screens/src/fixtures/build/waiting";

export const s160ChecksAgain: Scenario = holding("repair/checks-again", awaitingRepairChecksAgain().name, [awaitingRepairChecksAgain()], {
  opens: awaitingRepairChecksAgain().job.id,
});
