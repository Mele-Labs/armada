// The Board, with one Job at its review gate and a bell on its row: `wipe_qa` is on a destructive Command and
// blocks, so it waits on the owner before it runs. The Job opens to a leaf with Run and Skip. The mock Fleet
// answers them as Fleet does (`hold-fleet.ts`): Run passes the Command and the bell goes.

import { job2Asking } from "@armada/jobs/fake";

import { holding } from "../holding";
import type { Scenario } from "../moment";

const fixture = job2Asking();
const asking = holding("real/job-2-trigger-asks", "Job 2 at its review gate, a destructive Command waiting on the owner", [fixture]);

export const s025JobTwoAsks: Scenario = { ...asking, state: { ...asking.state, journalled: fixture.journalled } };
