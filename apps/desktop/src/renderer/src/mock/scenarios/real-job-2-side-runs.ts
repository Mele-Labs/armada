// Job 2 at its review gate with a Skill Trigger and a Drone step each running on a side Drone
// (23.73). The mock Fleet moves both as Fleet does (`repair-fleet.ts`): each Drone commits, its fix is
// held for the owner, and the choice places it with no Command to run again.

import { job2SideRuns } from "@armada/jobs/fake";

import { holding } from "../holding";
import type { Scenario } from "../moment";

const fixture = job2SideRuns();
const held = holding("real/job-2-side-runs", "Job 2 at its review gate, a Skill and a Drone step running", [fixture], { opens: fixture.job.id });

export const s024JobTwoSideRuns: Scenario = { ...held, state: { ...held.state, journalled: fixture.journalled } };
