// The canvas map's projection: where a point of the plane lands on the screen and the point a pixel stands for, which are each other's
// inverse on a level surface. The hit layer and the pointer rely on it: a pill under the pointer is found by running it backwards.

import { expect, test } from "vitest";

import { CAM } from "../layout";
import { easeOut, project, unproject, viewOf, type View } from "./project";

const PANE = { width: 900, height: 560 };
const views: [string, View][] = [
  ["wide, at rest", viewOf({ x: 0, y: 0, eye: 1, tilt: false }, PANE)],
  ["near, moved and close", viewOf({ x: 640, y: -220, eye: 2.2, tilt: true }, PANE)],
  ["between the two pitches", { ...viewOf({ x: -300, y: 410, eye: 0.7, tilt: false }, PANE), pitch: (CAM.wide + CAM.near) / 2 }],
];

test.each(views)("a point projected and run back is the point it was (%s)", (_, view) => {
  for (const z of [0, 14, 60]) {
    for (const [x, y] of [[0, 0], [300, 200], [-420, 330], [900, -150]] as const) {
      const at = project(view, { x, y, z });
      const back = unproject(view, at.x, at.y, z);
      expect(back.x).toBeCloseTo(x, 4);
      expect(back.y).toBeCloseTo(y, 4);
    }
  }
});

test("the point the camera looks at lands in the middle of the pane", () => {
  const view = viewOf({ x: 120, y: 80, eye: 1.5, tilt: true }, PANE);
  const at = project(view, { x: 120, y: 80, z: 0 });
  expect(at.x).toBeCloseTo(PANE.width / 2, 6);
  expect(at.y).toBeCloseTo(PANE.height / 2, 6);
  expect(at.scale).toBeCloseTo(1, 6);
});

test("what stands higher lands higher on the screen, and what is nearer the viewer is drawn larger", () => {
  const view = viewOf({ x: 0, y: 0, eye: 1, tilt: false }, PANE);
  expect(project(view, { x: 0, y: 0, z: 40 }).y).toBeLessThan(project(view, { x: 0, y: 0, z: 0 }).y);
  expect(project(view, { x: 0, y: 200, z: 0 }).scale).toBeGreaterThan(project(view, { x: 0, y: -200, z: 0 }).scale);
});

test("the camera's ease starts at 0, ends at 1 and only ever rises", () => {
  expect(easeOut(0)).toBe(0);
  expect(easeOut(1)).toBe(1);
  let last = 0;
  for (let t = 0.05; t < 1; t += 0.05) {
    const now = easeOut(t);
    expect(now).toBeGreaterThanOrEqual(last);
    last = now;
  }
});
