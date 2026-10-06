// The same Job just before it landed, at its review gate: the record the gate draws (#1680).

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { job2AtReview } from "@armada/jobs/fake";

export const s020JobTwoAtReview: Scenario = holding("real/job-2-at-review", job2AtReview().name, [job2AtReview()], { opens: job2AtReview().job.id });
