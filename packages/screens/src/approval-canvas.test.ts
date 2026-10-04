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
  it("runs brief to land: Groups after Implement, Done when before the pull request", () => {
    expect(nodesOf().nodes.map((node) => node.id)).toEqual([
      "brief",
      "base",
      "start",
      "plan",
      "plan:checks",
      "implement",
      "groups",
      "implement:checks",
      "done",
      "pr",
      "handoff",
      "land",
    ]);
  });

  it("puts Done when before Land where nothing opens a pull request", () => {
    const ids = nodesOf({ ...tuningOf([]), local: true }).nodes.map((node) => node.id);
    expect(ids.slice(-2)).toEqual(["done", "land"]);
  });

  it("draws the plan's groups in the placeholder's place, each in its own state", () => {
    const tuning = tuningOf([]);
    const life = {
      nodes: { plan: { activity: "advanced" as const, said: "advanced" } },
      groups: [
        { id: "G1", name: "Group 1", tasks: [], life: { activity: "advanced" as const, said: "landed" } },
        { id: "G2", name: "Group 2", tasks: [], life: { activity: "running" as const, said: "running", current: true } },
      ],
    };
    const { nodes } = approvalNodesOf({
      title: "T", from: "main", workflowName: "feature", steps: STEPS, gates: GATES, tuning, prMode: "ready", target: "main", life,
    });
    const ids = nodes.map((node) => node.id);
    expect(ids.slice(5, 8)).toEqual(["implement", "group:G1", "group:G2"]);
    expect(ids).not.toContain("groups");
    expect(nodes.find((node) => node.id === "plan")?.life?.activity).toBe("advanced");
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

describe("a wave's Jobs", () => {
  const EPIC: StepRead[] = [
    { id: "plan", label: "Plan the wave", checks: [], judges: [{ criteria: 2, gaming_check: false }], delivers: false },
    { id: "roll_up", label: "Roll up the wave", checks: [], judges: [], delivers: false },
  ];
  const gates = [
    { step_id: "plan", checks: false, judge: true, you: true },
    { step_id: "roll_up", checks: false, judge: false, you: true },
  ];
  const job = (id: string, waits_on: string[] = []) => ({
    id,
    name: id,
    waits_on,
    life: { activity: "running" as const, said: "running" },
  });
  const read = (jobs?: ReturnType<typeof job>[]) =>
    approvalNodesOf({
      title: "T", from: "main", workflowName: "epic", steps: EPIC, gates, tuning: tuningOf([]), prMode: "ready",
      target: "main", dispatchesFrom: "plan", ...(jobs === undefined ? {} : { life: { nodes: {}, jobs } }),
    });

  it("stands as Jobs after the step that dispatches them, until the wave exists", () => {
    expect(read().nodes.map((node) => node.id).slice(3, 6)).toEqual(["plan", "plan:checks", "jobs"]);
  });

  it("leads from the gate to each Job that waits on nothing, and on from each nobody waits on", () => {
    const { nodes, edges } = read([job("A"), job("B", ["A"]), job("C")]);
    expect(nodes.find((node) => node.id === "job:B")?.band).toEqual({ depth: 1, index: 0, of: 1 });
    const pairs = edges.map((edge) => `${edge.source}>${edge.target}`);
    expect(pairs).toContain("plan:checks>job:A");
    expect(pairs).toContain("plan:checks>job:C");
    expect(pairs).toContain("job:A>job:B");
    expect(pairs).toContain("job:B>roll_up");
    expect(pairs).toContain("job:C>roll_up");
    expect(pairs).not.toContain("job:A>roll_up");
  });
});
