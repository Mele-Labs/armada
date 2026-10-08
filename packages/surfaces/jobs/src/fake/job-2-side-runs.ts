// Job 2 at its review gate with a Skill Trigger and a Drone step each running on a side Drone
// (23.73): `tidy` names a skill and fires when the pull request opens, and the step added after
// `implement` carries a brief. Each has a branch of its own cut from the Job's, and each ends by
// holding what its Drone committed for the owner to place, as a repair's fix is held. Every row is
// shaped as Fleet serves it (`packages/protocol/src/triggers.ts`, `added-steps.ts`), and each move
// below is the row Fleet writes next (`fleet::side_run`, `fleet::placing_a_fix`).
//
// Invented: Job 2 was recorded before Triggers, so nothing here is something Fleet served.

import type { AddedStep, JobTrigger, TriggerFixChoice } from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { job2AtReviewWith } from "./job-2-at-review";
import { at, JOB_2_TRIGGERS } from "./job-2-triggers";

/** The Skill Trigger. */
export const SIDE_TRIGGER = "tidy";

/** The id of the step added after `implement`. */
export const SIDE_STEP = "a1";

export const SIDE_TRIGGER_BRANCH = "armada/run-tidy-1";
export const SIDE_STEP_BRANCH = "armada/run-a1-1";
export const SIDE_TRIGGER_FILES = ["docs/guides.md"];
export const SIDE_STEP_FILES = ["CHANGELOG.md"];
export const SIDE_PR_NUMBER = 1752;

/** `tidy` fired when the pull request opened, and a Drone is on it. */
const RUNNING_TRIGGER: JobTrigger = {
  name: SIDE_TRIGGER,
  when: "pr_opened",
  step: "handoff",
  level: "machine",
  state: "running",
  started_at: at(20),
  repair: { attempt: 1, branch: SIDE_TRIGGER_BRANCH },
  drone: true,
};

/** A Drone step added at approval, whose moment (`implement` passing) has come and whose Drone is at work. */
const RUNNING_STEP: AddedStep = {
  id: SIDE_STEP,
  runs: { kind: "drone", brief: "Add a changelog line" },
  when: "step_passes",
  step: "implement",
  block: false,
  repair: false,
  placed: "approval",
  added_at: at(5),
  state: "running",
  started_at: at(12),
  repair_record: { attempt: 1, branch: SIDE_STEP_BRANCH },
};

/** Job 2's Triggers, with `deploy_qa`'s place taken by the Skill Trigger. */
export const JOB_2_SIDE_RUNS: JobTrigger[] = JOB_2_TRIGGERS.map((one) => (one.name === "deploy_qa" ? RUNNING_TRIGGER : one));

/** The Job at its review gate with both Drones at work. */
export function job2SideRuns(): JobFixture {
  const fixture = job2AtReviewWith(JOB_2_SIDE_RUNS);
  const named = { ...fixture, name: "Job 2, at its review gate, a Skill and a Drone step running" };
  if (named.watched.state !== "read") return named;
  return { ...named, watched: { ...named.watched, detail: { ...named.watched.detail, additions: [RUNNING_STEP] } } };
}

/** The saved Trigger the `aDroneTrigger` walk makes: a prompt, named for its first words (23.75). */
export const DRONE_TRIGGER = "add-a-changelog-line";

/** Job 2 at its review gate with that Trigger fired when the pull request opened, its Drone at work. */
export function job2DroneTrigger(): JobFixture {
  const triggers = JOB_2_TRIGGERS.map((one) => (one.name === "deploy_qa" ? { ...RUNNING_TRIGGER, name: DRONE_TRIGGER } : one));
  return { ...job2AtReviewWith(triggers), name: "Job 2, at its review gate, a saved Drone Trigger running" };
}

/** The Drone committed: the Trigger's fix waits on the owner. */
export const sideFixHeld = (one: JobTrigger): JobTrigger => ({ ...one, state: "fix_ready", repair: { ...one.repair!, files: SIDE_TRIGGER_FILES } });

/** The Drone committed: the step's fix waits on the owner. */
export const sideStepFixHeld = (one: AddedStep): AddedStep => ({ ...one, state: "fix_ready", repair_record: { ...one.repair_record!, files: SIDE_STEP_FILES } });

const placedRecord = <T extends { choice?: TriggerFixChoice; pull_request?: { url: string; number?: number } }>(record: T, choice: TriggerFixChoice): T => ({
  ...record,
  choice,
  ...(choice === "new_pr" ? { pull_request: { url: `https://forge.test/pull/${SIDE_PR_NUMBER}`, number: SIDE_PR_NUMBER } } : {}),
});

/** The fix is placed, and there is no Command to run again: the Trigger passes. */
export const sidePlaced = (one: JobTrigger, choice: TriggerFixChoice, second: number): JobTrigger => ({
  ...one,
  state: "passed",
  ended_at: at(second),
  repair: placedRecord(one.repair!, choice),
});

export const sideStepPlaced = (one: AddedStep, choice: TriggerFixChoice, second: number): AddedStep => ({
  ...one,
  state: "passed",
  ended_at: at(second),
  repair_record: placedRecord(one.repair_record!, choice),
});
