// Steps added to one Job.
// A slice imports protocol and screens, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { AddingStep, AddStepAnswer, EditingStep, EditStepAnswer, ReadingRepairDiff, RemovingStep, RemoveStepAnswer, RepairDiffAnswer } from "../added-steps";

export type AddedStepsApi = {
  /**
   * Add a step to a Job that is underway, for that Job only. Fleet refuses a gap behind the Job's
   * current step (`fleet.added_step_behind`) and a Job not yet approved, whose approval carries it.
   * Removal is refused once the step has fired.
   */
  addJobStep: (adding: AddingStep) => Promise<AddStepAnswer>;
  removeJobStep: (removing: RemovingStep) => Promise<RemoveStepAnswer>;
  /** Change an added step's switches, refused once it has fired. */
  editJobStep: (editing: EditingStep) => Promise<EditStepAnswer>;
  /** What a repair or side-run fix changes, against the Job's branch. */
  readRepairDiff: (reading: ReadingRepairDiff) => Promise<RepairDiffAnswer>;
};

export type AddedStepsState = Record<never, never>;

export const ADDED_STEPS_NOTHING_YET: AddedStepsState = {};

export const ADDED_STEPS_CHANNELS = {
  addJobStep: "bridge:add-job-step",
  removeJobStep: "bridge:remove-job-step",
  editJobStep: "bridge:edit-job-step",
  readRepairDiff: "bridge:read-repair-diff",
} as const;
