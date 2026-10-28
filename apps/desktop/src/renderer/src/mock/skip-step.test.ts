// The rule for skipping a workflow step, as Fleet will hold it: where the Job stands after a skip or an undo, and which steps are refused.

import { describe, expect, it } from "vitest";

import { skipStep } from "./skip-step";

// Propose, Discuss, Approve plan, Walk, Refine, Deliver: the Job is at Walk (3) and Deliver (5) delivers.
const STEPS = 6;
const at = (now: number | undefined, skipped: number[] = []) => ({ at: now, skipped });

describe("skipping a step", () => {
  it("moves the Job past the current step it skips, to the next", () => {
    expect(skipStep(at(3), STEPS, 3, true)).toEqual({ at: 4, skipped: [3] });
  });

  it("moves the Job past a run of skipped steps, to the first one not skipped", () => {
    expect(skipStep(at(3, [4]), STEPS, 3, true)).toEqual({ at: 5, skipped: [3, 4] });
  });

  it("only marks a step to come, and the Job stays where it is", () => {
    expect(skipStep(at(3), STEPS, 4, true)).toEqual({ at: 3, skipped: [4] });
  });

  it("keeps the skipped steps in order, and a step skipped twice once", () => {
    expect(skipStep(at(1, [4]), STEPS, 2, true)).toEqual({ at: 1, skipped: [2, 4] });
    expect(skipStep(at(3, [4]), STEPS, 4, true)).toEqual({ at: 3, skipped: [4] });
  });

  it("leaves the stand it was given as it was", () => {
    const stand = at(3, [4]);
    skipStep(stand, STEPS, 3, true);
    expect(stand).toEqual({ at: 3, skipped: [4] });
  });

  it("is a Job with no step to stand at, and no step is moved", () => {
    expect(skipStep(at(undefined), STEPS, 2, true)).toEqual({ at: undefined, skipped: [2] });
  });
});

describe("undoing a skip", () => {
  it("unmarks a step to come, and the Job stays where it is", () => {
    expect(skipStep(at(3, [4]), STEPS, 4, false)).toEqual({ at: 3, skipped: [] });
  });

  it("puts the Job back at the step it skipped when only skipped steps lie between", () => {
    expect(skipStep(at(5, [3, 4]), STEPS, 3, false)).toEqual({ at: 3, skipped: [4] });
  });

  it("puts the Job back at the step just before the current one", () => {
    expect(skipStep(at(4, [3]), STEPS, 3, false)).toEqual({ at: 3, skipped: [] });
  });

  it("leaves the Job where it is when a step it did not skip lies between", () => {
    expect(skipStep(at(5, [3]), STEPS, 3, false)).toEqual({ at: 5, skipped: [] });
  });

  it("changes nothing for a step that was not skipped", () => {
    expect(skipStep(at(3, [4]), STEPS, 2, false)).toEqual({ at: 3, skipped: [4] });
  });
});

describe("a step that cannot be skipped", () => {
  it("is the last, delivering step, to skip or to undo", () => {
    expect(skipStep(at(3), STEPS, 5, true)).toBeUndefined();
    expect(skipStep(at(3), STEPS, 5, false)).toBeUndefined();
  });

  it("is a step past the workflow's end, or one before its start", () => {
    expect(skipStep(at(3), STEPS, 6, true)).toBeUndefined();
    expect(skipStep(at(3), STEPS, -1, true)).toBeUndefined();
  });

  it("is a step that is not a whole number", () => {
    expect(skipStep(at(3), STEPS, 3.5, true)).toBeUndefined();
    expect(skipStep(at(3), STEPS, Number.NaN, true)).toBeUndefined();
  });

  it("is any step of a workflow with one step", () => {
    expect(skipStep(at(0), 1, 0, true)).toBeUndefined();
  });
});
