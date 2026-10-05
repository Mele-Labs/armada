// Where a running Job is on its canvas: past steps done, the one it is on
// current, and nothing reached past it.

import type { JobDetail as JobWhole } from "@armada/protocol";
import { describe, expect, it } from "vitest";

import { lifeOf } from "./approval-life";

const whole = {
  job: { current_step_id: "implement" },
  steps: [
    { step_id: "plan", state: "advanced", judge_checks: [{ criteria: 1, gaming_check: false }] },
    { step_id: "implement", state: "running" },
    { step_id: "handoff", state: "not_started" },
  ],
} as unknown as JobWhole;

describe("lifeOf", () => {
  it("marks the step the Job is on current, and what it passed done", () => {
    const { nodes, groups } = lifeOf(whole);
    expect(nodes["brief"]?.activity).toBe("advanced");
    expect(nodes["plan"]?.activity).toBe("advanced");
    expect(nodes["plan:judge"]?.activity).toBe("advanced");
    expect(nodes["implement"]).toMatchObject({ activity: "running", said: "running", current: true });
    expect(nodes["implement:checks"]).toBeUndefined();
    expect(nodes["handoff"]?.current).toBeUndefined();
    // No plan recorded, so the canvas keeps its Groups placeholder.
    expect(groups).toBeUndefined();
  });

  it("keeps a step's Checks as passed after the gate was asked again", () => {
    // Asking again records attempt 2 and re-runs no Check, so `check_runs` still names attempt 1.
    const asked = {
      job: { current_step_id: "plan" },
      steps: [
        {
          step_id: "plan",
          state: "stopped",
          checks: [{ kind: "plan_recorded" }],
          check_runs: [{ attempt: 1, name: "plan_recorded", outcome: "passed" }],
          attempts: [
            { attempt: 1, outcome: "stopped", why: "gate_undecided", started_at: "2026-10-05T14:00:00Z" },
            { attempt: 2, outcome: "stopped", why: "gate_undecided", started_at: "2026-10-05T14:10:00Z" },
          ],
        },
      ],
    } as unknown as JobWhole;
    expect(lifeOf(asked).nodes["plan:checks"]).toMatchObject({
      activity: "advanced",
      run: { commands: [{ name: "plan_recorded", outcome: "passed" }] },
    });
  });
});
