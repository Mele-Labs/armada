// The Judges' risk reason the call card says, read off a Job's detail: only the step it is at, and only
// that step's latest round of answers. #2105.

import { describe, expect, it } from "vitest";
import type { JobDetail, Judged, StepDetail } from "@armada/protocol";

import { riskOf } from "./judges-risk";

const judged = (attempt: number, verdict: string, risky?: string): Judged => ({ attempt, criterion_id: "c1", verdict, ...(risky === undefined ? {} : { risky }) }) as Judged;

function detail(current: string, steps: Record<string, Judged[]>): JobDetail {
  return {
    job: { current_step_id: current },
    steps: Object.entries(steps).map(([step_id, answers]) => ({ step_id, judged: answers }) as unknown as StepDetail),
  } as unknown as JobDetail;
}

describe("the Judges' risk reason", () => {
  it("is the risky member's reason on the step the Job is at", () => {
    const read = detail("review", { review: [judged(1, "not_met"), judged(1, "not_met", " It rewrites the migration history. ")] });
    expect(riskOf(read)).toBe("It rewrites the migration history.");
  });

  it("is only the latest round's, and never a met answer's", () => {
    expect(riskOf(detail("review", { review: [judged(1, "not_met", "Earlier worry."), judged(2, "not_met")] }))).toBeUndefined();
    expect(riskOf(detail("review", { review: [judged(1, "met", "Said, but it passed.")] }))).toBeUndefined();
  });

  it("is nothing from a step the Job has left", () => {
    expect(riskOf(detail("land", { review: [judged(1, "not_met", "Risky.")], land: [] }))).toBeUndefined();
  });
});
