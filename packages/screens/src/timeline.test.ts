// One run's turns, case by case: an attempt's window ends where the next one
// began.
import { describe, expect, it } from "vitest";

import type { StepDetail } from "@armada/protocol";

import { freshStep, said } from "./fixtures/build/base";
import { turnsOfAttempt } from "./timeline";

/** A step on its second attempt, the first handed back by a Check. */
function handedBack(over: Partial<StepDetail> = {}): StepDetail {
  return {
    ...freshStep("regression_verify", "Regression check", 4),
    state: "running",
    entered_at: "2026-09-10T14:22:18Z",
    updated_at: "2026-09-10T14:26:00Z",
    attempts: [
      {
        attempt: 1,
        outcome: "retrying",
        why: "gate_failure",
        started_at: "2026-09-10T14:22:18Z",
        ended_at: "2026-09-10T14:24:40Z",
      },
      { attempt: 2, outcome: "running", started_at: "2026-09-10T14:24:40Z" },
    ],
    ...over,
  };
}

describe("one run's turns, for a sheet opened from it", () => {
  const first = said("regression_verify", "2026-09-10T14:23:30Z", "first");
  const second = said("regression_verify", "2026-09-10T14:25:30Z", "second");

  it("ends an attempt where the next one began", () => {
    expect(turnsOfAttempt(handedBack(), 1, [first, second])).toEqual([first]);
    expect(turnsOfAttempt(handedBack(), 2, [first, second])).toEqual([second]);
  });

  it("answers with the step's whole record for an attempt it does not have", () => {
    // Wider than asked beats a reading that silently empties.
    expect(turnsOfAttempt(handedBack(), 9, [first, second])).toEqual([first, second]);
  });
});
