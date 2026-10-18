// Where each thing a person can press or hover is on the canvas, as a box (and for a ring's piece or a bay's pad, the shape) in the pixels of
// the pane. The DOM that stands over the canvas is put there from these each time the view moves (`Hits.tsx`). Every size is a hit box scaled the way the canvas scales the picture beneath it, so a pointer lands on what it looks at. Pure.

import type { P3, Point } from "../layout";
import { onPlane } from "../bf-parts";
import { project, type View } from "./project";
import { OUTPOST_SCALE, type DroneSpec, type Placed, type WorkerSpec } from "./scene";
import { workerAt } from "./workers";

/** A box in the pane, and where it is narrower than its box, the outline inside it (an SVG path in the box's own pixels). */
export type Spot = { x: number; y: number; w: number; h: number; shape?: string };

type Screen = { x: number; y: number; scale: number };

/** A box beside a point of the scene, its sides given as the offsets a picture of that scene would carry them at, scaled by `s`. */
const beside = (at: Screen, s: number, left: number, top: number, right: number, bottom: number): Spot => ({ x: at.x + left * s, y: at.y + top * s, w: (right - left) * s, h: (bottom - top) * s });

/** The outline of some rings of points, as the smallest box round them and a path inside it. */
function outline(rings: readonly (readonly Screen[])[]): Spot {
  const all = rings.flat();
  const [x0, y0] = [Math.min(...all.map((p) => p.x)), Math.min(...all.map((p) => p.y))];
  const [x1, y1] = [Math.max(...all.map((p) => p.x)), Math.max(...all.map((p) => p.y))];
  const path = rings.map((ring) => `${ring.map((p, i) => `${i === 0 ? "M" : "L"}${(p.x - x0).toFixed(1)} ${(p.y - y0).toFixed(1)}`).join(" ")} Z`).join(" ");
  return { x: x0, y: y0, w: Math.max(1, x1 - x0), h: Math.max(1, y1 - y0), shape: path };
}

/** A drone's hit box: round its body. */
export function droneSpot(view: View, spec: DroneSpec, big: number): Spot | undefined {
  const here = project(view, { x: spec.at.x, y: spec.at.y, z: spec.alt });
  if (here.scale <= 0) return undefined;
  const wide = 20;
  return beside(here, view.eye * here.scale * big, -(wide + 12), -34, wide + 12, 22);
}

/** The foot of the stem the marks over a drone stand on, and the scale they are drawn at (the zoom does not change their size). */
function markFoot(view: View, spec: DroneSpec, big: number): { x: number; y: number; scale: number } | undefined {
  const here = project(view, { x: spec.at.x, y: spec.at.y, z: spec.alt });
  if (here.scale <= 0) return undefined;
  return { x: here.x, y: here.y - Math.round(40 * big) * view.eye * here.scale, scale: here.scale };
}

/** The badge of a call it holds. */
export function callSpot(view: View, spec: DroneSpec, big: number): Spot | undefined {
  const foot = markFoot(view, spec, big);
  return foot === undefined ? undefined : beside({ x: foot.x, y: foot.y - 33 * foot.scale, scale: 1 }, foot.scale, -17, -17, 17, 17);
}

/** The mark of the `i`th write clash it is in, beside the call badge. */
export function clashSpot(view: View, spec: DroneSpec, i: number, big: number): Spot | undefined {
  const foot = markFoot(view, spec, big);
  return foot === undefined ? undefined : beside({ x: foot.x + (32 + 30 * i) * foot.scale, y: foot.y - 13 * foot.scale, scale: 1 }, foot.scale, -13, -13, 13, 13);
}

const CIRCLE = 20;

/** A circle lying on the ground at `z`, as the tilted ellipse it is on the screen. */
function ground(view: View, centre: Point, r: number, z: number): Screen[] {
  return Array.from({ length: CIRCLE }, (_, i) => project(view, { x: centre.x + r * Math.cos((i / CIRCLE) * 2 * Math.PI), y: centre.y + r * Math.sin((i / CIRCLE) * 2 * Math.PI), z }));
}

/** A bay's pad: a ring of 17 on the ground. */
export const padSpot = (view: View, at: Point): Spot => outline([ground(view, at, 17, 0)]);

/** The flag that is main: the box round its pole and pennant. */
export function goalSpot(view: View, goal: Point): Spot | undefined {
  const here = project(view, { ...onPlane(goal), z: 1 });
  return here.scale <= 0 ? undefined : beside(here, view.eye * here.scale, -14, -84, 78, 6);
}

/** A service outpost: the box round its badge, mast and slab. */
export function outpostSpot(view: View, at: Point): Spot | undefined {
  const here = project(view, { ...at, z: 0 });
  return here.scale <= 0 ? undefined : beside(here, view.eye * here.scale, -26 * OUTPOST_SCALE, -68 * OUTPOST_SCALE, 26 * OUTPOST_SCALE, 12 * OUTPOST_SCALE);
}

/** A pull request's crate, at rest on its route. */
export function crateSpot(view: View, at: P3): Spot | undefined {
  const here = project(view, at);
  return here.scale <= 0 ? undefined : beside(here, view.eye * here.scale, -13, -13, 13, 13);
}

/** A subagent: the CSS map's box round the small drone, which the zoom scales and the Session's size does not. */
export function workerSpot(view: View, w: WorkerSpec, floor: number): Spot | undefined {
  const here = project(view, workerAt(w, floor));
  return here.scale <= 0 ? undefined : beside(here, view.eye * here.scale, -14, -12, 14, 8);
}

// ---- A ring ------------------------------------------------------------------------------------------------

/** The sizes a ring is drawn at, in the plane's units. */
export function ringOf(big: number): { r0: number; r1: number; a0: number; a1: number } {
  const r1 = 34 * big;
  return { r0: r1 - 7 * big, r1, a0: r1 + 4 * big, a1: r1 + 8.5 * big };
}

/** An annular sector between two radii and two angles in degrees, as the points of its edge. */
function sector(view: View, centre: Point, inner: number, outer: number, from: number, to: number, z: number): Screen[] {
  const steps = Math.max(3, Math.ceil(Math.abs(to - from) / 12));
  const at = (r: number, deg: number) => project(view, { x: centre.x + r * Math.cos((deg * Math.PI) / 180), y: centre.y + r * Math.sin((deg * Math.PI) / 180), z });
  return [...Array.from({ length: steps + 1 }, (_, i) => at(outer, from + ((to - from) * i) / steps)), ...Array.from({ length: steps + 1 }, (_, i) => at(inner, to - ((to - from) * i) / steps))];
}

/** Where a ring lies: round its anchor, at the height of the plot under it. */
const floorOf = (one: Placed, heights: ReadonlyMap<string, number>) => (one.tile === undefined ? 0 : (heights.get(one.tile) ?? 0)) + 1.5;

/** The `k`th of `n` pieces of the ring of tasks. */
export function pieceSpot(view: View, one: Placed, heights: ReadonlyMap<string, number>, k: number, n: number, big: number): Spot {
  const { r0, r1 } = ringOf(big);
  const centre = onPlane(one.anchor);
  return outline([sector(view, centre, r0, r1, -90 + (k * 360) / n + 2.5, -90 + ((k + 1) * 360) / n - 2.5, floorOf(one, heights))]);
}

/** The `k`th of `m` arcs of the road outside it. */
export function arcSpot(view: View, one: Placed, heights: ReadonlyMap<string, number>, k: number, m: number, big: number): Spot {
  const { a0, a1 } = ringOf(big);
  return outline([sector(view, onPlane(one.anchor), a0, a1, -90 + (k * 360) / m + 3, -90 + ((k + 1) * 360) / m - 3, floorOf(one, heights))]);
}

/** The one band that stands for a ring with no list of tasks: the annulus, with its middle left open. */
export function bandSpot(view: View, one: Placed, heights: ReadonlyMap<string, number>, big: number): Spot {
  const { r0, r1 } = ringOf(big);
  const [centre, z] = [onPlane(one.anchor), floorOf(one, heights)];
  return outline([ground(view, centre, r1, z), ground(view, centre, r0, z)]);
}

/** A line between two drones changing one area, thickened to a strip a pointer can find: the points of the line, and a few pixels either side. */
export function lineSpot(view: View, line: readonly P3[], half: number): Spot | undefined {
  const points = line.map((p) => project(view, p));
  if (points.some((p) => p.scale <= 0) || points.length < 2) return undefined;
  const side = points.map((_, i) => {
    const [a, b] = [points[Math.max(0, i - 1)]!, points[Math.min(points.length - 1, i + 1)]!];
    const length = Math.hypot(b.x - a.x, b.y - a.y) || 1;
    return { nx: (-(b.y - a.y) / length) * half, ny: ((b.x - a.x) / length) * half };
  });
  return outline([[...points.map((p, i) => ({ x: p.x + side[i]!.nx, y: p.y + side[i]!.ny, scale: 1 })), ...points.map((p, i) => ({ x: p.x - side[i]!.nx, y: p.y - side[i]!.ny, scale: 1 })).reverse()]]);
}
