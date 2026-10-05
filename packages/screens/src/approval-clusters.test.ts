import { describe, expect, it } from "vitest";

import type { ApprovalNode } from "./approval-canvas";
import { layoutOf } from "./approval-layout";

const base = { lane: "work" as const, traits: [], meta: [], ordinal: 0 };
const step: ApprovalNode = { ...base, id: "implement", kind: "step", name: "Implement" };
const gate: ApprovalNode = { ...base, id: "implement:checks", kind: "checks", name: "Checks" };
const group = (id: string): ApprovalNode => ({ ...base, id: `group:${id}`, kind: "group", name: id, from: "implement" });
const task = (id: string, of: string, wave: number): ApprovalNode => ({
  ...base,
  id: `task:${id}`,
  kind: "task",
  name: id,
  chain: { group: `group:${of}`, at: 0, wave },
});

// G1 holds a wave of two; G2 a chain of two.
const nodes = [step, group("G1"), task("T1", "G1", 0), task("T2", "G1", 0), group("G2"), task("T3", "G2", 0), task("T4", "G2", 1), gate];

describe("the clusters layout", () => {
  it("sizes a group as the frame its tasks sit in, side by side with the next", () => {
    const { places, sizes } = layoutOf(nodes, []);
    const one = sizes.get("group:G1")!;
    const two = sizes.get("group:G2")!;
    expect(one.width).toBeGreaterThan(two.width);
    expect(places.get("group:G1")!.x + one.width).toBeLessThan(places.get("group:G2")!.x);
    expect(places.get("group:G1")!.y).toBe(places.get("group:G2")!.y);
  });

  it("holds each task inside its cluster, concurrent ones on one row and the rest below", () => {
    const { places, sizes } = layoutOf(nodes, []);
    const at = (id: string) => places.get(id)!;
    expect(at("task:T1").y).toBe(at("task:T2").y);
    expect(at("task:T4").y).toBeGreaterThan(at("task:T3").y);
    const frame = { ...at("group:G2"), ...sizes.get("group:G2")! };
    expect(at("task:T4").y).toBeGreaterThan(frame.y);
    expect(at("task:T4").y).toBeLessThan(frame.y + frame.height);
    expect(at("task:T3").x).toBeGreaterThanOrEqual(frame.x);
  });

  it("puts the gate below the tallest cluster, and draws no frames of its own for the groups", () => {
    const { places, sizes, frames } = layoutOf(nodes, []);
    const tallest = Math.max(...[...sizes.values()].map((one) => one.height));
    expect(places.get("implement:checks")!.y).toBeGreaterThan(places.get("group:G1")!.y + tallest);
    expect(frames.some((one) => one.kind === "cluster")).toBe(false);
  });

  it("gives the clusters in a row one height, the tallest", () => {
    const { sizes } = layoutOf(nodes, []);
    expect(sizes.get("group:G1")!.height).toBe(sizes.get("group:G2")!.height);
  });
});
