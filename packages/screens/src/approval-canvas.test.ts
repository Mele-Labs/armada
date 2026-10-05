// The run the approval canvas draws: a step's gate as its own node where it
// declares something to run, and the end reshaped by how the work lands.

import { describe, expect, it } from "vitest";

import { approvalNodesOf, stepsReadOf } from "./approval-canvas";
import type { StepRead } from "./approval-canvas";
import { tuningOf } from "./draft/tuning";

const STEPS: StepRead[] = [
  { id: "plan", label: "Plan the change", checks: [], judges: [{ criteria: 2, gaming_check: false }], delivers: false, phase: "work" as const, perTask: false, dispatches: false },
  { id: "implement", label: "Implement", checks: [{ kind: "diff_nonempty" }], judges: [], delivers: false, phase: "work" as const, perTask: true, dispatches: false },
  { id: "handoff", label: "Review the change", checks: [], judges: [], delivers: true, phase: "delivery" as const, perTask: false, dispatches: false },
];

const GATES = [
  { step_id: "plan", checks: false, judge: true, you: false },
  { step_id: "implement", checks: true, judge: false, you: false },
  { step_id: "handoff", checks: false, judge: false, you: true },
];

const nodesOf = (
  tuning = tuningOf(STEPS.map((step) => ({ step_id: step.id }))),
  prMode: "ready" | "draft" = "ready",
  delivery: { local?: boolean; autoMerge?: boolean } = {},
) => approvalNodesOf({ title: "T", from: "main", steps: STEPS, gates: GATES, tuning, prMode, ...delivery, target: "main" });

describe("approvalNodesOf", () => {
  it("runs brief to land: Groups after Implement, Done when before the pull request", () => {
    expect(nodesOf().nodes.map((node) => node.id)).toEqual([
      "brief",
      "base",
      "plan",
      "plan:judge",
      "implement",
      "groups",
      "implement:checks",
      "done",
      "pr",
      "handoff",
      "handoff:you",
      "land",
    ]);
  });

  it("puts Done when before the Review step and its You stage where nothing opens a pull request", () => {
    const { nodes } = nodesOf(tuningOf([]), "ready", { local: true });
    expect(nodes.map((node) => node.id).slice(-4)).toEqual(["done", "handoff", "handoff:you", "land"]);
  });

  it("lays the run in three lanes, each gate a stage on its lane's spine", () => {
    const { nodes } = nodesOf();
    const lane = (id: string) => nodes.find((node) => node.id === id)?.lane;
    expect([lane("base"), lane("plan"), lane("done"), lane("land")]).toEqual(["setup", "work", "delivery", "delivery"]);
    expect(nodes.some((node) => node.id === "start")).toBe(false);
    expect(lane("plan:judge")).toBe("work");
    expect(lane("handoff:you")).toBe("delivery");
    expect(nodes.find((node) => node.id === "plan:judge")?.gate).toBe("judge");
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
      title: "T", from: "main", steps: STEPS, gates: GATES, tuning, prMode: "ready", target: "main", life,
    });
    const ids = nodes.map((node) => node.id);
    expect(ids.slice(4, 7)).toEqual(["implement", "group:G1", "group:G2"]);
    expect(ids).not.toContain("groups");
    expect(nodes.find((node) => node.id === "plan")?.life?.activity).toBe("advanced");
  });

  it("draws no pull request where it lands locally", () => {
    const { nodes } = nodesOf(tuningOf([]), "ready", { local: true });
    expect(nodes.some((node) => node.kind === "pr")).toBe(false);
    // Local only holds the work on its branch: no merge, so none is said.
    expect(nodes.at(-1)?.meta).toEqual([
      { key: "No pull request, no merge, no push: the work stays on its branch", value: "local only", tuned: true },
    ]);
    expect(nodes.at(-1)?.face).toBeUndefined();
  });

  it("says on the edge into Land that it merges on its own", () => {
    const { edges } = nodesOf(tuningOf([]), "draft", { autoMerge: true });
    expect(edges.at(-1)?.label).toBe("merges on its own");
  });
});

describe("a wave's Jobs", () => {
  const EPIC: StepRead[] = [
    { id: "plan", label: "Plan the wave", checks: [], judges: [{ criteria: 2, gaming_check: false }], delivers: false, phase: "work" as const, perTask: false, dispatches: false },
    { id: "roll_up", label: "Roll up the wave", checks: [], judges: [], delivers: false, phase: "work" as const, perTask: false, dispatches: false },
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
      title: "T", from: "main", steps: EPIC, gates, tuning: tuningOf([]), prMode: "ready",
      target: "main", dispatchesFrom: "plan", ...(jobs === undefined ? {} : { life: { nodes: {}, jobs } }),
    });

  it("stands as Jobs after the step that dispatches them, until the wave exists", () => {
    const placeholder = read().nodes.find((node) => node.id === "jobs");
    expect(placeholder?.lane).toBe("work");
    expect(read().nodes.map((node) => node.id).slice(2, 6)).toEqual(["plan", "plan:judge", "plan:you", "jobs"]);
  });

  it("leads from the gate to each Job that waits on nothing, and on from each nobody waits on", () => {
    const { nodes, edges } = read([job("A"), job("B", ["A"]), job("C")]);
    expect(nodes.find((node) => node.id === "job:B")?.band).toEqual({ depth: 1, index: 0, of: 1 });
    const pairs = edges.map((edge) => `${edge.source}>${edge.target}`);
    expect(pairs).toContain("plan:judge>plan:you");
    expect(pairs).toContain("plan:you>job:A");
    expect(pairs).toContain("plan:you>job:C");
    expect(pairs).toContain("job:A>job:B");
    expect(pairs).toContain("job:B>roll_up");
    expect(pairs).toContain("job:C>roll_up");
    expect(pairs).not.toContain("job:A>roll_up");
  });
});

describe("a step's lane", () => {
  it("is its declared phase: an Epic's roll-up, which delivers nothing, is delivery", () => {
    const steps: StepRead[] = [
      { id: "plan", label: "Plan the wave", checks: [], judges: [], delivers: false, phase: "work", perTask: false, dispatches: false },
      { id: "roll_up", label: "Roll up the wave", checks: [], judges: [], delivers: false, phase: "delivery", perTask: false, dispatches: false },
    ];
    const gates = [
      { step_id: "plan", checks: false, judge: false, you: true },
      { step_id: "roll_up", checks: false, judge: false, you: true },
    ];
    const { nodes } = approvalNodesOf({
      title: "T", from: "main", steps, gates, tuning: tuningOf([]), prMode: "ready", target: "main",
    });
    expect(nodes.find((node) => node.id === "roll_up")?.lane).toBe("delivery");
    expect(nodes.find((node) => node.id === "plan")?.lane).toBe("work");
  });
});

describe("the Studio node", () => {
  it("comes first, in setup, only where the Job came from a Studio", () => {
    const tuning = tuningOf([]);
    const base = { title: "T", from: "main", steps: STEPS, gates: GATES, tuning, prMode: "ready" as const, target: "main" };
    const from = approvalNodesOf({ ...base, studio: "Error contract" }).nodes;
    expect(from[0]).toMatchObject({ id: "studio", lane: "setup", face: "Error contract", opensStudio: true });
    expect(from[1]?.id).toBe("brief");
    expect(approvalNodesOf(base).nodes.some((node) => node.kind === "studio")).toBe(false);
  });
});

describe("a setting moved since the approval", () => {
  const life = {
    nodes: { plan: { activity: "advanced" as const, said: "advanced" } },
    changed: { model: "opus", whenRefused: { now: "Always stop the step", was: "Ask me" } },
  };
  const { nodes } = approvalNodesOf({
    title: "T", from: "main", steps: STEPS, gates: GATES, tuning: tuningOf([]), prMode: "ready", target: "main", life,
  });
  const node = (id: string) => nodes.find((one) => one.id === id)!;

  it("is drawn on the steps still to run, in accent, with what it was in the tooltip", () => {
    expect(node("implement").traits).toEqual([{ key: "Model, was Auto", value: "opus", tuned: true }]);
    // Plan has run, so a model chosen since does not reach it.
    expect(node("plan").traits).toEqual([]);
  });

  it("puts what a refusing Judge does on the Judge's gates still to run, and nowhere else", () => {
    const judged = GATES.map((gate) => (gate.step_id === "implement" ? { ...gate, judge: true } : gate));
    const steps = STEPS.map((step) => (step.id === "implement" ? { ...step, judges: [{ criteria: 1, gaming_check: false }] } : step));
    const read = approvalNodesOf({
      title: "T", from: "main", steps, gates: judged, tuning: tuningOf([]), prMode: "ready", target: "main", life,
    }).nodes;
    const gate = (id: string) => read.find((one) => one.id === id)!;
    expect(gate("implement:judge").meta).toEqual([
      { key: "When the Judge refuses, was Ask me", value: "Always stop the step", tuned: true },
    ]);
    // Plan's gate has passed, so the change does not reach it.
    expect(gate("plan:judge").meta).toEqual([]);
  });
});

describe("Land, local only", () => {
  it("names the branch the work stays on, where the Job has one", () => {
    const { nodes } = approvalNodesOf({
      title: "T", from: "main", steps: STEPS, gates: GATES, tuning: tuningOf([]),
      prMode: "ready", local: true, target: "main", branch: "armada/1-retire-guide-8",
    });
    expect(nodes.at(-1)).toMatchObject({ id: "land", face: "armada/1-retire-guide-8" });
  });
});

describe("stepsReadOf", () => {
  const gates = [{ step_id: "handoff", checks: false, judge: true, you: true }];
  const whole = { steps: [{ step_id: "handoff", label: "Review", judge_checks: [{ criteria: 1, gaming_check: false }] }] } as never;
  const catalog = new Map([["handoff", { step_id: "handoff", label: "Review", judge_checks: [] } as never]]);

  it("reads the catalog at the gate, and the Job's own step past it", () => {
    expect(stepsReadOf(gates, whole, catalog)[0]?.judges).toEqual([]);
    expect(stepsReadOf(gates, whole, catalog, true)[0]?.judges).toHaveLength(1);
  });
});
