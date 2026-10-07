// Steps added to one Job.
// A slice imports protocol and screens, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { AddingStep, AddStepAnswer, RemovingStep, RemoveStepAnswer } from "../added-steps";

export type AddedStepsApi = {
  /**
   * Add a step to a Job that is underway, for that Job only. Fleet refuses a gap behind the Job's
   * current step (`fleet.added_step_behind`) and a Job not yet approved, whose approval carries it.
   * Removal is refused once the step has fired.
   */
  addJobStep: (adding: AddingStep) => Promise<AddStepAnswer>;
  removeJobStep: (removing: RemovingStep) => Promise<RemoveStepAnswer>;
};

export type AddedStepsState = Record<never, never>;

export const ADDED_STEPS_NOTHING_YET: AddedStepsState = {};

export const ADDED_STEPS_CHANNELS = {
  addJobStep: "bridge:add-job-step",
  removeJobStep: "bridge:remove-job-step",
} as const;
