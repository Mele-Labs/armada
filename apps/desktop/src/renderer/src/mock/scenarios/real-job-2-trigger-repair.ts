// Job 2 at its review gate, with `deploy_qa` failed after the pull request opened and a repair Drone
// at work on a branch of its own. The mock Fleet moves the row as Fleet does (`repair-fleet.ts`): the
// fix is held for the owner, and the Trigger passes once he has chosen where it goes.

import { job2Repairing } from "@armada/jobs/fake";

import { holding } from "../holding";
import type { Scenario } from "../moment";

const fixture = job2Repairing();
const held = holding("real/job-2-trigger-repair", "Job 2 at its review gate, a failed Trigger being repaired", [fixture], { opens: fixture.job.id });

// The Job's own log is on a socket of its own, which the mock holds as published state.
export const s021JobTwoRepairing: Scenario = { ...held, state: { ...held.state, journalled: fixture.journalled } };
