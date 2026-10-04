// The run the approval canvas draws: a step's gate as its own node where it
// declares something to run, and the end reshaped by how the work lands.

import { describe, expect, it } from "vitest";

import { approvalNodesOf } from "./approval-canvas";
import type { StepRead } from "./approval-canvas";
import { tuningOf } from "./draft/tuning";

const STEPS: StepRead[] = [
  { id: "plan", label: "Plan the change", checks: [], judges: [{ criteria: 2, gaming_check: false }], delivers: false },
  { id: "implement", label: "Implement", checks: [{ kind: "diff_nonempty" }], judges: [], delivers: false },
  { id: "handoff", label: "Review the change", checks: [], judges: [], delivers: true },
];

const GATES = [
  { step_id: "plan", checks: false, judge: true, you: false },
  { step_id: "implement", checks: true, judge: false, you: false },
  { step_id: "handoff", checks: false, judge: false, you: true },
];

const nodesOf = (tuning = tuningOf(STEPS.map((step) => ({ step_id: step.id }))), prMode: "ready" | "draft" = "ready") =>
  approvalNodesOf({ title: "T", from: "main", workflowName: "feature", steps: STEPS, gates: GATES, tuning, prMode, target: "main" });

describe("approvalNodesOf", () => {
  it("runs brief to land, the pull request before the step that delivers", () => {
    expect(nodesOf().nodes.map((node) => node.id)).toEqual([
      "brief",
      "base",
      "start",
      "plan",
      "plan:checks",
      "implement",
      "implement:checks",
      "pr",
      "handoff",
      "land",
    ]);
  });

  it("draws no pull request where it lands locally", () => {
    const tuning = { ...tuningOf([]), local: true };
    const { nodes } = nodesOf(tuning);
    expect(nodes.some((node) => node.kind === "pr")).toBe(false);
    expect(nodes.at(-1)?.facts).toEqual([{ value: "Local merge" }]);
  });

  it("says on the edge into Land that it merges on its own", () => {
    const { edges } = nodesOf({ ...tuningOf([]), auto_merge: true }, "draft");
    expect(edges.at(-1)?.label).toBe("merges on its own");
  });
});
