// Job 2 at its review gate with a blocking `deploy_qa` failed after the pull request opened, so it holds
// the Job. The mock Fleet answers Rerun and Skip as Fleet does (`hold-fleet.ts`): a rerun passes and lets
// the hold go, and the Job's row loses its alert.

import { job2Held } from "@armada/jobs/fake";

import { holding } from "../holding";
import type { Scenario } from "../moment";

const fixture = job2Held();
const held = holding("real/job-2-trigger-hold", "Job 2 at its review gate, held by a Trigger", [fixture], { opens: fixture.job.id });

export const s023JobTwoHeld: Scenario = { ...held, state: { ...held.state, journalled: fixture.journalled } };
