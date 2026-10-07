// The same Job under an id of its own, with a repair that finds no fix in either try: the Trigger fails
// for good and the Job has an alert.

import { job2RepairFails } from "@armada/jobs/fake";

import { holding } from "../holding";
import type { Scenario } from "../moment";

const fixture = job2RepairFails();
const held = holding("real/job-2-trigger-repair-fails", "Job 2 at its review gate, a Trigger's repair finding no fix", [fixture], { opens: fixture.job.id });

export const s022JobTwoRepairFails: Scenario = { ...held, state: { ...held.state, journalled: fixture.journalled } };
