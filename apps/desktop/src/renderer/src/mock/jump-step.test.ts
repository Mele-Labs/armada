// The rule for jumping to a workflow step, as Fleet will hold it: which steps are skipped and where the Job lands.

import { describe, expect, it } from "vitest";

import { jumpStep } from "./jump-step";

// Propose, Discuss, Approve plan, Walk, Refine, Deliver: the Job is at Discuss (1).
const STEPS = 6;
const at = (now: number | undefined, skipped: number[] = []) => ({ at: now, skipped });

describe("jumping to a step", () => {
  it("skips every step from the current one up to the one before the target", () => {
    expect(jumpStep(at(1), STEPS, 4)).toEqual({ at: 4, skipped: [1, 2, 3] });
  });

  it("keeps the steps skipped before, in order, and lists none twice", () => {
    expect(jumpStep(at(1, [3, 5]), STEPS, 4)).toEqual({ at: 4, skipped: [1, 2, 3, 5] });
  });

  it("lifts a skip of the step jumped to", () => {
    expect(jumpStep(at(1, [3]), STEPS, 3)).toEqual({ at: 3, skipped: [1, 2] });
  });

  it("may land on the last step", () => {
    expect(jumpStep(at(1), STEPS, 5)).toEqual({ at: 5, skipped: [1, 2, 3, 4] });
  });

  it("refuses the current step, an earlier one, one past the last and a half step", () => {
    expect(jumpStep(at(1), STEPS, 1)).toBeUndefined();
    expect(jumpStep(at(1), STEPS, 0)).toBeUndefined();
    expect(jumpStep(at(1), STEPS, 6)).toBeUndefined();
    expect(jumpStep(at(1), STEPS, 2.5)).toBeUndefined();
  });

  it("refuses a Job that is on no step", () => {
    expect(jumpStep(at(undefined), STEPS, 3)).toBeUndefined();
  });

  it("leaves the stand it was given as it was", () => {
    const stand = at(1, [3]);
    jumpStep(stand, STEPS, 4);
    expect(stand).toEqual({ at: 1, skipped: [3] });
  });
});
