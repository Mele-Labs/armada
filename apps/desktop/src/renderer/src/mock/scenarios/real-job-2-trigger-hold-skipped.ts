// The same held Job under an id of its own, where the owner skips the Trigger instead of rerunning it.

import { job2HeldAndSkipped } from "@armada/jobs/fake";

import { holding } from "../holding";
import type { Scenario } from "../moment";

const fixture = job2HeldAndSkipped();
const held = holding("real/job-2-trigger-hold-skipped", "Job 2 at its review gate, held by a Trigger the owner skips", [fixture], { opens: fixture.job.id });

export const s024JobTwoHeldAndSkipped: Scenario = { ...held, state: { ...held.state, journalled: fixture.journalled } };
