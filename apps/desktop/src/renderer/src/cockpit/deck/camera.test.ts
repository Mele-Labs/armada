// The map's camera, as pure readings. The claims are the ones the owner walked: it rests zoomed out with the whole field in the
// pane, the wheel and a pinch zoom toward the cursor, a drag moves the ground with the pointer, it never leaves the field, and a
// call that is out of the pane is pointed at from its border.

import { expect, test } from "vitest";

import { FLAT, type P3 } from "./layout";
import { KEY_PAN, ZOOM_STEP, edgeOf, homeOf, limitsOf, panned, projectOf, unproject, zoomed, type Cam } from "./camera";

const SIZE = { width: 1000, height: 640 };
const PLANE = { width: 1800, height: 1150 * FLAT };
const at = (x: number, y: number, z = 0): P3 => ({ x, y: y * FLAT, z });

/** Where a point lands on the screen, from the pane's middle, as the camera draws it. */
const shown = (p: P3, cam: Cam) => projectOf(SIZE, p, cam);

test("unprojecting a screen point on the ground undoes the projection, at any eye and either pitch", () => {
  for (const tilt of [false, true]) {
    for (const eye of [0.5, 1, 2.2]) {
      const cam: Cam = { x: 700, y: 500, eye, tilt };
      const p = { x: 760, y: 560, z: 0 };
      const there = shown(p, cam);
      const back = unproject(SIZE, there.x, there.y, tilt);
      // What `unproject` gives is in plane units at an eye of one, from the camera's own point.
      expect(back.x / eye).toBeCloseTo(p.x - cam.x, 1);
      expect(back.y / eye).toBeCloseTo(p.y - cam.y, 1);
    }
  }
});

test("the camera rests with every mark of the field inside the pane, with room at the edges", () => {
  const marks: P3[] = [at(100, 100), at(1700, 100), at(100, 1050), at(1700, 1050), at(900, 80, 78), at(950, 600, 120), at(900, 1090, 40)];
  const home = homeOf(SIZE, PLANE, marks);

  for (const mark of marks) {
    const there = shown(mark, home);
    expect(Math.abs(there.x)).toBeLessThanOrEqual(SIZE.width / 2);
    expect(Math.abs(there.y)).toBeLessThanOrEqual(SIZE.height / 2);
  }
  // Fitted, not merely inside: the marks fill most of one of the two directions.
  const xs = marks.map((mark) => shown(mark, home).x);
  const ys = marks.map((mark) => shown(mark, home).y);
  const filled = Math.max((Math.max(...xs) - Math.min(...xs)) / SIZE.width, (Math.max(...ys) - Math.min(...ys)) / SIZE.height);
  expect(filled).toBeGreaterThan(0.85);
  expect(home.tilt).toBe(false);
});

test("the field is fitted whatever the pane's size is", () => {
  const marks: P3[] = [at(100, 100), at(1700, 1050), at(900, 80, 78)];
  for (const size of [{ width: 520, height: 700 }, { width: 1400, height: 500 }, { width: 900, height: 900 }]) {
    const home = homeOf(size, PLANE, marks);
    for (const mark of marks) {
      const there = projectOf(size, mark, home);
      expect(Math.abs(there.x)).toBeLessThanOrEqual(size.width / 2);
      expect(Math.abs(there.y)).toBeLessThanOrEqual(size.height / 2);
    }
  }
});

test("with nothing to show the camera rests on the middle of the plane", () => {
  const home = homeOf(SIZE, PLANE, []);
  expect(home.x).toBe(PLANE.width / 2);
  expect(home.y).toBe(PLANE.height / 2);
});

test("zooming toward the pointer keeps the ground under it where it was", () => {
  const marks: P3[] = [at(100, 100), at(1700, 1050)];
  const home = homeOf(SIZE, PLANE, marks);
  const limits = limitsOf(SIZE, PLANE, PLANE.width, home);
  const pointer = { x: 210, y: -90 };
  const before: Cam = { ...home, eye: Math.max(home.eye * 1.6, limits.least * 1.2) };
  const under = unproject(SIZE, pointer.x, pointer.y, before.tilt);
  const ground = { x: before.x + under.x / before.eye, y: before.y + under.y / before.eye, z: 0 };

  const after = zoomed(SIZE, PLANE, limits, before, ZOOM_STEP, pointer)!;
  expect(after.eye).toBeGreaterThan(before.eye);
  const now = shown(ground, after);
  expect(now.x).toBeCloseTo(pointer.x, 0);
  expect(now.y).toBeCloseTo(pointer.y, 0);
});

test("the eye is held between the whole field and as close as a drone is large, and a zoom past either is no zoom", () => {
  const home = homeOf(SIZE, PLANE, [at(100, 100), at(1700, 1050)]);
  const limits = limitsOf(SIZE, PLANE, PLANE.width, home);
  expect(limits.least).toBeLessThanOrEqual(home.eye);
  expect(limits.most).toBeGreaterThan(2);

  expect(zoomed(SIZE, PLANE, limits, { ...home, eye: limits.most }, ZOOM_STEP, { x: 0, y: 0 })).toBeUndefined();
  expect(zoomed(SIZE, PLANE, limits, { ...home, eye: limits.least }, 1 / ZOOM_STEP, { x: 0, y: 0 })).toBeUndefined();
  const huge = zoomed(SIZE, PLANE, limits, home, 40, { x: 0, y: 0 })!;
  expect(huge.eye).toBe(limits.most);
});

test("a drag carries the ground with the pointer, and a pan never leaves the field", () => {
  const home = homeOf(SIZE, PLANE, [at(100, 100), at(1700, 1050)]);
  const limits = limitsOf(SIZE, PLANE, PLANE.width, home);
  const cam: Cam = { ...home, eye: Math.min(limits.most, home.eye * 2) };
  // The ground under the pointer at the start of a drag is under it at the end.
  const start = { x: 40, y: 30 };
  const end = { x: -120, y: 90 };
  const grabbed = unproject(SIZE, start.x, start.y, cam.tilt);
  const ground = { x: cam.x + grabbed.x / cam.eye, y: cam.y + grabbed.y / cam.eye, z: 0 };
  const from = unproject(SIZE, start.x, start.y, cam.tilt);
  const to = unproject(SIZE, end.x, end.y, cam.tilt);
  const dragged = { ...cam, x: cam.x + (from.x - to.x) / cam.eye, y: cam.y + (from.y - to.y) / cam.eye };
  expect(shown(ground, dragged).x).toBeCloseTo(end.x, 0);
  expect(shown(ground, dragged).y).toBeCloseTo(end.y, 0);

  const far = panned(SIZE, PLANE, limits, cam, 1e6, -1e6);
  expect(far.x).toBeGreaterThanOrEqual(0);
  expect(far.x).toBeLessThanOrEqual(PLANE.width);
  expect(far.y).toBeGreaterThanOrEqual(0);
  expect(far.y).toBeLessThanOrEqual(PLANE.height);
});

test("an arrow key pans by a fixed step of the screen", () => {
  const home = homeOf(SIZE, PLANE, [at(100, 100), at(1700, 1050)]);
  const limits = limitsOf(SIZE, PLANE, PLANE.width, home);
  const cam: Cam = { ...home, eye: 1.2, x: PLANE.width / 2, y: PLANE.height / 2 };
  const moved = panned(SIZE, PLANE, limits, cam, KEY_PAN, 0);
  // Across the screen, give or take the yaw of the ground.
  expect(moved.x - cam.x).toBeGreaterThan(5 * Math.abs(moved.y - cam.y));
});

test("a call inside the pane has no edge marker, and one outside has it on the border pointing at it", () => {
  const cam: Cam = { x: 900, y: 575 * FLAT, eye: 1, tilt: false };
  const inside = edgeOf(SIZE, cam, at(900, 575, 40));
  expect(inside).toBeUndefined();

  const right = edgeOf(SIZE, cam, at(2400, 575, 40))!;
  expect(right.x).toBeGreaterThan(SIZE.width / 2);
  expect(right.x).toBeLessThanOrEqual(SIZE.width);
  expect(Math.abs(right.angle)).toBeLessThan(40);

  const above = edgeOf(SIZE, cam, at(900, -900, 40))!;
  expect(above.y).toBeLessThan(SIZE.height / 2);
  expect(above.y).toBeGreaterThanOrEqual(0);
  // Pointing up the screen.
  expect(above.angle).toBeLessThan(-45);
  expect(above.angle).toBeGreaterThan(-135);
});
