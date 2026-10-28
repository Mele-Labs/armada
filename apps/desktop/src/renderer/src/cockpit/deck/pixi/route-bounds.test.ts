// A pull request's route to a point far off the field: the dashed stroke once grew its vertices until the array could hold no more, and
// the whole overview fell to "Invalid array length". A route like that is left out, and no stroke grows without bound.

import { expect, test } from "vitest";

import { frameOf } from "../frame";
import { bigOf, layoutOf, type PrIn } from "../layout";
import { arcOf } from "./air";
import { PlaneBuilder } from "./plane";
import { DRONES, PANE, REPO, sceneFor } from "./scene-fixture";

const floats = (b: PlaneBuilder) => (b.geometry().attributes.aPosition!.buffer.data as Float32Array).length;

test("a dashed stroke to a point at infinity or far off the field returns with a bounded count of vertices", () => {
  for (const x of [Infinity, 1e8]) {
    const b = new PlaneBuilder();
    b.stroke(arcOf({ x: 0, y: 0, z: 0 }, { x, y: 0, z: 0 }), 0.8, [1, 1, 1, 1], { dash: [2.4, 5.6] });
    expect(floats(b)).toBeLessThan(2_000_000);
  }
});

test("a dashed stroke of an ordinary length is still cut into dashes", () => {
  const b = new PlaneBuilder();
  b.stroke([{ x: 0, y: 0, z: 0 }, { x: 80, y: 0, z: 0 }], 0.8, [1, 1, 1, 1], { dash: [2.4, 5.6] });
  // Ten dashes of two points, two vertices each, ten floats a vertex.
  expect(floats(b)).toBe(10 * 2 * 2 * 10);
});

test("a pull request whose owner stands far off the field is left out of the crates, and one near it is kept", () => {
  const world = { width: PANE.width * 1.6, height: PANE.height * 1.6 };
  const big = bigOf(PANE);
  const prs: PrIn[] = [{ key: "pr:7", number: 7, stage: "ready", root: "/r", owner: "session:Writer" } as PrIn];
  const layout = layoutOf([REPO], DRONES, world, big, prs, []);
  const frame = frameOf({ layout, drones: DRONES, plays: [], big });
  expect(sceneFor({ prs, layout, frame }).crates.map((one) => one.pr.number)).toEqual([7]);
  for (const far of [{ x: 1e8, y: 0, z: 40 }, { x: Infinity, y: 0, z: 40 }, { x: 0, y: -Infinity, z: 40 }]) {
    expect(sceneFor({ prs, layout, frame: { ...frame, flying: () => far } }).crates).toEqual([]);
  }
});
