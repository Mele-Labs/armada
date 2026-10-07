// Job 2 at its review gate with a failed Trigger being repaired (7 Oct 2026): `deploy_qa` failed
// after the pull request opened, and Fleet put a repair Drone on a branch of its own. Every row is
// shaped as `JobDetail.triggers` carries it (`packages/protocol/src/triggers.ts`), and each move
// below is the row Fleet writes next (`fleet::repairing`, `fleet::placing_a_fix`).
//
// Invented: Job 2 was recorded before Triggers, so nothing here is something Fleet served.

import type { JobTrigger, TriggerFixChoice } from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { job2AtReviewWith } from "./job-2-at-review";
import { at, JOB_2_TRIGGERS } from "./job-2-triggers";

/** The repair branch, cut from the Job's. */
export const REPAIR_BRANCH = "armada/repair-deploy_qa-1";

/** What the fix changes, which is what the owner reads before he chooses. */
export const REPAIR_FILES = ["deploy/qa.sh", ".armada/qa.env"];

/** The number the forge gave the pull request a `new_pr` choice opens. */
export const REPAIR_PR_NUMBER = 1751;

/** The Trigger a repair is on. */
export const REPAIRED = "deploy_qa";

/** `deploy_qa` has failed and the first repair Drone is at work. **Not ended**: the repair settles it. */
const FAILED_AND_BEING_REPAIRED: JobTrigger = {
  name: REPAIRED,
  when: "pr_opened",
  step: "handoff",
  level: "machine",
  state: "repairing",
  exit_code: 1,
  started_at: at(20),
  log_at: at(20),
  repair: { attempt: 1, branch: REPAIR_BRANCH },
};

/** Job 2's Triggers, with `deploy_qa` where the repair has it. */
export const JOB_2_REPAIRING: JobTrigger[] = JOB_2_TRIGGERS.map((one) => (one.name === REPAIRED ? FAILED_AND_BEING_REPAIRED : one));

/** The Job at its review gate with `deploy_qa` being repaired. */
export function job2Repairing(): JobFixture {
  return { ...job2AtReviewWith(JOB_2_REPAIRING), name: "Job 2, at its review gate, a failed Trigger being repaired" };
}

/** The same Job under an id of its own, so a repair that finds nothing is its own. */
export const repairFailsId = (): string => `${job2Repairing().job.id.slice(0, -1)}N`;

export function job2RepairFails(): JobFixture {
  const was = job2Repairing();
  const fixture = JSON.parse(JSON.stringify(was).replaceAll(was.job.id, repairFailsId())) as JobFixture;
  return { ...fixture, name: "Job 2, at its review gate, a Trigger's repair finding no fix" };
}

/** The repair Drone's second try, after the Command failed again on the first. */
export const secondTry = (one: JobTrigger): JobTrigger => ({ ...one, repair: { ...one.repair!, attempt: 2 } });

/** The Command passes on the repair branch, and the fix waits on the owner. */
export const fixHeld = (one: JobTrigger): JobTrigger => ({
  ...one,
  state: "fix_ready",
  repair: { ...one.repair!, files: REPAIR_FILES },
});

/** Both tries failed: the Trigger is `failed` for good, which is the Job's alert. */
export const noFix = (one: JobTrigger, second: number): JobTrigger => ({
  ...one,
  state: "failed",
  ended_at: at(second),
  repair: { ...secondTry(one).repair!, files: [] },
});

/** The owner chose `this_branch`: the fix is merged and the Command runs again on the Job's branch. */
export const chosenOntoTheBranch = (one: JobTrigger): JobTrigger => ({
  ...one,
  state: "rerunning",
  repair: { ...one.repair!, choice: "this_branch" },
});

/** The fix is placed and the Trigger passes. `new_pr` carries the pull request it opened. */
export const placed = (one: JobTrigger, choice: TriggerFixChoice, second: number): JobTrigger => ({
  ...one,
  state: "passed",
  exit_code: 0,
  ended_at: at(second),
  repair: {
    ...one.repair!,
    choice,
    ...(choice === "new_pr" ? { pull_request: { url: `https://forge.test/pull/${REPAIR_PR_NUMBER}`, number: REPAIR_PR_NUMBER } } : {}),
  },
});
