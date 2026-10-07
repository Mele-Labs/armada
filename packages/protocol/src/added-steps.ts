// Steps added to one Job: placed at approval, added to a Job underway, removed before they fire,
// and how each stands. Hand-mirrored from `crates/ipc/src/added_steps.rs`. Since protocol 23.59.
//
// An addition sits beside the Job's frozen workflow and never in it, so it is on `JobDetail.additions`
// and not in the workflow's steps.

import type { TriggerFiringState, TriggerMoment, TriggerScope } from "./triggers";

/**
 * What an added step runs. A Skill is recorded `skipped`, as a skill Trigger is, and so is a Drone
 * step: a step a Drone works needs a gate Fleet does not have beside the workflow's.
 */
export type AddedRuns =
  | { kind: "script"; command: string }
  | { kind: "skill"; skill: string }
  | { kind: "drone"; brief: string };

/**
 * One step to add. A place is a moment and a step, a Trigger's: before a step is `step_starts`,
 * after it is `step_passes`, and `pr_opened` hangs from the delivering step. `approve_dispatch`'s
 * `additions` and `add_job_step`'s body.
 */
export type AddStep = {
  runs: AddedRuns;
  when: TriggerMoment;
  step: string;
  /** Carried and not acted on yet. Absent is off. */
  block?: boolean;
  /** Carried and not acted on yet. Absent is off. */
  repair?: boolean;
};

export type RemoveAddedStep = { id: string };

export type AddedStepRemoved = { id: string };

export type AddedPlaced = "approval" | "running";

export type AddedSkipReason = "not_in_this_repo" | "skill_not_run" | "drone_step_not_run";

/** `said` is rendered and never matched on. */
export type AddedSkip = { reason: AddedSkipReason; said: string };

/** One step added to a Job and how it stands. `pending` until its moment has come, which is when it can still be removed. */
export type AddedStep = {
  /** `a1`, `a2` and so on, in the order they were added to the Job. */
  id: string;
  runs: AddedRuns;
  when: TriggerMoment;
  step: string;
  block: boolean;
  repair: boolean;
  placed: AddedPlaced;
  added_at: string;
  state: TriggerFiringState;
  skipped?: AddedSkip;
  exit_code?: number;
  started_at?: string;
  ended_at?: string;
  /** The Job's log line for the latest firing: the note whose `at` is this and whose `addition` field is `id`. */
  log_at?: string;
  /** Where it was kept for every Job, once it was. */
  kept?: TriggerScope;
};

/** `save_trigger`'s `kept_from`: the addition the save keeps for every Job. A Script or a Skill only. */
export type KeptFrom = { job_id: string; addition_id: string };

/** `job.addition_changed`: one row, whole. `removed` is true for the row as it was when it was taken off. */
export type JobAdditionChanged = { job_id: string; addition: AddedStep; removed?: boolean; at: string };
