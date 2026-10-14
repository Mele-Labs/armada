// What a scene says about the order it plays in: a step on an arrow, beside the one on a node. The format is
// `packages/components/src/compositions/SketchScene/scene.ts`; Fleet's validator for `show_sketch` has to agree with it.

import { describe, expect, it } from "vitest";

import { diffScenes, parseScene } from "@armada/components/src/compositions/SketchScene/scene";

const nodes = [
  { id: "a", kind: "box", x: 0, y: 0, title: "Bridge" },
  { id: "b", kind: "box", x: 300, y: 0, title: "Fleet" },
];

describe("a step on an arrow", () => {
  it("is read, and kept with the arrow", () => {
    const scene = parseScene({ nodes, edges: [{ id: "a-b", from: "a", to: "b", step: 2 }] });
    expect(scene?.edges).toEqual([{ id: "a-b", from: "a", to: "b", step: 2 }]);
  });

  it("is optional: an arrow with none stays still and is read as it always was", () => {
    const scene = parseScene({ nodes, edges: [{ id: "a-b", from: "a", to: "b", label: "reads", flow: false }] });
    expect(scene?.edges).toEqual([{ id: "a-b", from: "a", to: "b", label: "reads", flow: false }]);
  });

  it("fails the whole scene when it is not a number", () => {
    expect(parseScene({ nodes, edges: [{ id: "a-b", from: "a", to: "b", step: "1" }] })).toBeUndefined();
    expect(parseScene({ nodes, edges: [{ id: "a-b", from: "a", to: "b", step: Number.NaN }] })).toBeUndefined();
  });

  it("is a change to the arrow where it moves, as a node's step is", () => {
    const before = parseScene({ nodes, edges: [{ id: "a-b", from: "a", to: "b", step: 1 }] })!;
    const after = parseScene({ nodes, edges: [{ id: "a-b", from: "a", to: "b", step: 2 }] })!;
    expect(diffScenes(before, after).edges.get("a-b")).toBe("changed");
    expect(diffScenes(before, before).edges.get("a-b")).toBe("same");
  });
});
