// Job 2 at its review gate, with a trigger that failed after the PR opened and a
// repair Drone that finds no fix in either try: the other end of the repair
// branch. The same Job under an id of its own, so its repair is its own.

import type { JobFixture } from "@armada/screens/src/fixtures/fixture";
import { job2AtReview } from "@armada/jobs/fake";

import { holding } from "../holding";
import type { Scenario } from "../moment";

const ORIGINAL = job2AtReview().job.id;
const FAILS = `${ORIGINAL.slice(0, -1)}N`;

function failing(): JobFixture {
  return JSON.parse(JSON.stringify(job2AtReview()).replaceAll(ORIGINAL, FAILS)) as JobFixture;
}

export const s021JobTwoRepairFails: Scenario = holding("real/job-2-trigger-repair-fails", "Job 2 at its review gate, a trigger's repair finding no fix", [failing()], { opens: FAILS });
