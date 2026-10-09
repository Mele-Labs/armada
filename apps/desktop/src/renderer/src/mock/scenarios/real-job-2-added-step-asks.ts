// The Board, with one Job at its review gate and a bell on its row: `wipe_qa` is a Script added to this Job
// only, on a destructive Command, and it blocks, so it waits on the owner before it runs. The Job opens to
// a leaf with Run and Skip. The mock Fleet answers them as Fleet does (`hold-fleet.ts`): Run passes the
// Command and the bell goes.

import { job2AddedAsking } from "@armada/jobs/fake";

import { holding } from "../holding";
import type { Scenario } from "../moment";

const fixture = job2AddedAsking();
const asking = holding("real/job-2-added-step-asks", "Job 2 at its review gate, an added step on a destructive Command waiting on the owner", [fixture]);

export const s027JobTwoAddedAsks: Scenario = { ...asking, state: { ...asking.state, journalled: fixture.journalled } };
