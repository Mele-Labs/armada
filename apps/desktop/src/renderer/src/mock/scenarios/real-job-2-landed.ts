// The owner's Job 2 as `GET /jobs/2` served it: four groups Bridge stood in
// for, every task still `open`, and a 40-character commit.

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { job2Landed } from "@armada/jobs/fake";

export const s010JobTwoLanded: Scenario = holding("real/job-2-landed", job2Landed().name, [job2Landed()], { opens: job2Landed().job.id });
