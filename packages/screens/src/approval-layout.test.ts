// The approval canvas laid in lanes: a gate's stages under their step, a fan under its
// step, every lane as tall as the deepest, and an edge across lanes turning in
// the gutter between them.

import { describe, expect, it } from "vitest";

import { approvalNodesOf } from "./approval-canvas";
import type { StepRead } from "./approval-canvas";
import { layoutOf } from "./approval-layout";
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

const run = () => {
  const life = {
    nodes: {},
    groups: ["G1", "G2"].map((id) => ({ id, name: id, tasks: [], life: { activity: "not_started" as const, said: "" } })),
  };
  const { nodes, edges } = approvalNodesOf({
    title: "T", from: "main", steps: STEPS, gates: GATES, tuning: tuningOf([]),
    prMode: "ready", target: "main", life,
  });
  return { nodes, ...layoutOf(nodes, edges) };
};

describe("layoutOf", () => {
  it("stands a gate's stage on the spine under its step, close under it", () => {
    const { places } = run();
    // Inset: narrower than the step, on the same spine.
    expect(places.get("plan:judge")!.x + 228 / 2).toBe(places.get("plan")!.x + 260 / 2);
    const gap = places.get("plan:judge")!.y - places.get("plan")!.y - 112;
    expect(gap).toBeGreaterThan(0);
    expect(gap).toBeLessThan(places.get("implement")!.y - places.get("plan:judge")!.y - 104);
  });

  it("fans a step's groups across one row under it, and frames them", () => {
    const { places, frames } = run();
    expect(places.get("group:G1")?.y).toBe(places.get("group:G2")?.y);
    expect(places.get("group:G1")!.y).toBeGreaterThan(places.get("implement")!.y);
    expect(frames.some((frame) => frame.id === "cluster:implement" && frame.kind === "cluster")).toBe(true);
  });

  it("draws the three lanes left to right, each as tall as the deepest", () => {
    const { places, frames } = run();
    expect(places.get("base")!.x).toBeLessThan(places.get("plan")!.x);
    expect(places.get("plan")!.x).toBeLessThan(places.get("done")!.x);
    const zones = frames.filter((frame) => frame.kind === "zone");
    expect(zones.map((zone) => zone.name)).toEqual(["Setup", "Work", "Delivery"]);
    expect(new Set(zones.map((zone) => zone.height)).size).toBe(1);
  });

  it("turns an edge across lanes in the gutter left of the lane it enters", () => {
    const { edges, frames } = run();
    const work = frames.find((frame) => frame.id === "zone:work")!;
    const setup = frames.find((frame) => frame.id === "zone:setup")!;
    const into = edges.find((edge) => edge.source === "base" && edge.target === "plan");
    expect(into?.via).toBeGreaterThan(setup.x + setup.width);
    expect(into?.via).toBeLessThan(work.x);
    expect(edges.find((edge) => edge.source === "plan" && edge.target === "plan:judge")?.via).toBeUndefined();
  });
});
