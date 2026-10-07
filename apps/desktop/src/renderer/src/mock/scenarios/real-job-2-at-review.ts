// The same Job just before it landed, at its review gate: the record the gate draws (#1680).

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { job2AtReview } from "@armada/jobs/fake";

const held = holding("real/job-2-at-review", job2AtReview().name, [job2AtReview()], { opens: job2AtReview().job.id });

// The Job's own log is on a socket of its own, which the mock holds as published state: it is what
// the Triggers' lines are found in.
export const s020JobTwoAtReview: Scenario = { ...held, state: { ...held.state, journalled: job2AtReview().journalled } };
