// Where a running Job is on its canvas: past steps done, the one it is on
// current, and nothing reached past it.

import type { JobDetail as JobWhole } from "@armada/protocol";
import { describe, expect, it } from "vitest";

import { lifeOf } from "./approval-life";

const whole = {
  job: { current_step_id: "implement" },
  steps: [
    { step_id: "plan", state: "advanced" },
    { step_id: "implement", state: "running" },
    { step_id: "handoff", state: "not_started" },
  ],
} as unknown as JobWhole;

describe("lifeOf", () => {
  it("marks the step the Job is on current, and what it passed done", () => {
    const { nodes, groups } = lifeOf(whole);
    expect(nodes["brief"]?.activity).toBe("advanced");
    expect(nodes["plan"]?.activity).toBe("advanced");
    expect(nodes["plan:checks"]?.activity).toBe("advanced");
    expect(nodes["implement"]).toMatchObject({ activity: "running", said: "running", current: true });
    expect(nodes["implement:checks"]).toBeUndefined();
    expect(nodes["handoff"]?.current).toBeUndefined();
    // No plan recorded, so the canvas keeps its Groups placeholder.
    expect(groups).toBeUndefined();
  });
});
