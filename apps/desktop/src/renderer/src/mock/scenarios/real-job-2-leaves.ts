// Job 2 at its review gate with a leaf of every kind off the step it fired at: a Trigger that passed,
// one that failed, a repair's branch holding its fix, and a Skill and a Drone step each on a Drone.

import { job2Leaves } from "@armada/jobs/fake";
import { repository } from "@armada/screens/src/fixtures/build/base";

import { holding } from "../holding";
import type { Scenario } from "../moment";

const fixture = job2Leaves();
const held = holding("real/job-2-leaves", "Job 2 at its review gate, a leaf of every kind off its step", [fixture], { opens: fixture.job.id, alsoServed: [repository()] });

export const s025JobTwoLeaves: Scenario = {
  ...held,
  // The repository is picked so the Workflows page, one press away on the rail, has its workflows to read.
  state: { ...held.state, journalled: fixture.journalled, repository: repository().root },
};
