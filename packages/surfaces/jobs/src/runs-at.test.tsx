// Where a Check runs, from the screen's side. #849.
//
// A green run is not the whole bar when some Checks never ran in it, so a
// handoff Check the gate has not reached says it runs last.

import { describe, expect, it } from "vitest";

import type { CheckRun as CheckRunRow } from "@armada/components";
import type { StepDetail } from "@armada/protocol";

import { checkRow } from "./checks";
import { RUNS_LAST_BEFORE_HANDOFF } from "./declared";
import { checksOf } from "./gates";

const AT = "2026-09-14T09:00:00Z";

function step(over: Partial<StepDetail> = {}): StepDetail {
  return {
    step_id: "tests",
    label: "Write tests",
    ordinal: 2,
    state: "running",
    checks: [
      { kind: "manifest_check", name: "build", run: "cargo build" },
      { kind: "manifest_check", name: "storybook", run: "pnpm build-storybook", runs_at: "gate" },
      { kind: "manifest_check", name: "e2e", run: "pnpm e2e", runs_at: "handoff" },
    ],
    check_runs: [],
    overridden: false,
    judged: [],
    flagged: [],
    attempts: [],
    verdicts: [],
    entered_at: AT,
    updated_at: AT,
    ...over,
  };
}

function rowsOf(detail: StepDetail): CheckRunRow[] {
  return checksOf(detail).map((read) => checkRow(read, Date.parse(AT)));
}

describe("the gate's reading", () => {
  it("says a handoff Check it has not reached runs last", () => {
    const e2e = rowsOf(step()).find((row) => row.id === "e2e");
    expect(e2e?.says).toBe(RUNS_LAST_BEFORE_HANDOFF);
    expect(e2e?.named).toBe("queued");
  });
});
