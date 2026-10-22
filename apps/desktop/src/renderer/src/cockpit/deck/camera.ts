// The map's camera: where it looks on the plane and how far it stands, and the maths the pointer, the wheel and the keys move it by.
// Pure. The canvas applies `rotateX(pitch) rotateZ(yaw) scale(eye) translate(-x, -y)` in a perspective (`pixi/project.ts`), and these readings are that
// same projection and its inverse, so a point held under the pointer stays under it however far from the middle it is.
//
// **Rest is the whole field, zoomed out** (`homeOf`): every territory, base, flag and drone inside the pane at whatever size the
// pane is. A hand on the camera is a different camera, which `limitsOf` holds between the whole field and as close as a drone is
// large. Ported from proto/cockpit-flight-deck. #2301.

import { CAM, FLAT, SLAB, type P3, type Point, type Size } from "./layout";

/** Where the camera looks (on the plane, the depth already stretched by `FLAT`), how close it stands, and whether it is the lower, nearer pitch. */
export type Cam = { x: number; y: number; eye: number; tilt: boolean };

/** How much larger the field is than the pane, each way. */
export const WORLD = 1.8;
/** The share of the pane left clear round the field at home, each side. */
export const HOME_MARGIN = 0.035;
/** Below this zoom the type is too small to read: a name is dropped and the callout names it. */
export const READABLE = 0.72;
/** How close lock-on stands. */
export const LOCK_EYE = 1.5;
/** How much one press of + or - zooms, and how far an arrow key pans, in screen units. */
export const ZOOM_STEP = 1.25;
export const KEY_PAN = 90;
/** A press that moves less than this is a click, not a drag. */
export const DRAG = 4;
/** How far from the pane's border a call's edge marker stands. */
export const EDGE_ROOM = 26;
/** The least zoom a pane may ever be held at, and the closest. */
const LEAST = 0.3;
const MOST = 2.6;

const COS = Math.cos((CAM.wide * Math.PI) / 180);
const rad = (deg: number) => (deg * Math.PI) / 180;

/** The depth of the perspective, in the pane's own units. */
const depthOf = (size: Size) => size.width * 2.8;

/**
 * Where a screen offset from the middle of the pane lies on the plane, from the aim, in plane units at an eye of one: the camera's
 * own projection undone (the yaw, the tilt and the perspective).
 */
export function unproject(size: Size, sx: number, sy: number, tilt: boolean): Point {
  const phi = rad(tilt ? CAM.near : CAM.wide);
  const yaw = rad(CAM.yaw);
  const depth = depthOf(size);
  const y1 = (sy * depth) / (depth * Math.cos(phi) + sy * Math.sin(phi));
  const x1 = (sx * (depth - y1 * Math.sin(phi))) / depth;
  return { x: x1 * Math.cos(yaw) + y1 * Math.sin(yaw), y: -x1 * Math.sin(yaw) + y1 * Math.cos(yaw) };
}

/** Where a point of the plane, at some height, lands on the screen, from the middle of the pane: the same turn, scale and perspective the canvas applies. */
export function projectOf(size: Size, p: P3, c: Cam): Point {
  const phi = rad(c.tilt ? CAM.near : CAM.wide);
  const yaw = rad(CAM.yaw);
  const [dx, dy, z] = [(p.x - c.x) * c.eye, (p.y - c.y) * c.eye, p.z * c.eye];
  const ux = dx * Math.cos(yaw) - dy * Math.sin(yaw);
  const uy = dx * Math.sin(yaw) + dy * Math.cos(yaw);
  const depth = depthOf(size);
  const scale = depth / (depth - (uy * Math.sin(phi) + z * Math.cos(phi)));
  return { x: ux * scale, y: (uy * Math.cos(phi) - z * Math.sin(phi)) * scale };
}

/**
 * The camera at rest: the whole field in the pane with a small margin, found from the projection itself so that the perspective's
 * growth toward the viewer is allowed for. `marks` are the points that must show, at their heights.
 */
export function homeOf(size: Size, plane: Size, marks: readonly P3[] | ((eye: number) => readonly P3[]), within?: Box, most = Infinity): Cam {
  const at = (eye: number) => (typeof marks === "function" ? marks(eye) : marks);
  const first = at(0.6);
  if (first.length === 0) return { x: plane.width / 2, y: plane.height / 2, eye: 0.8, tilt: false };
  // The part of the pane the marks are fitted into, as offsets from its middle: the whole pane unless a box is given.
  const box = within ?? { l: 0, t: 0, r: size.width, b: size.height };
  const [bx, by] = [(box.l + box.r) / 2 - size.width / 2, (box.t + box.b) / 2 - size.height / 2];
  let cam: Cam = { x: first.reduce((sum, m) => sum + m.x, 0) / first.length, y: first.reduce((sum, m) => sum + m.y, 0) / first.length, eye: 0.6, tilt: false };
  for (let pass = 0; pass < (within === undefined ? 8 : 12); pass += 1) {
    const there = at(cam.eye).map((m) => projectOf(size, m, cam));
    const [l, r] = [Math.min(...there.map((m) => m.x)), Math.max(...there.map((m) => m.x))];
    const [t, b] = [Math.min(...there.map((m) => m.y)), Math.max(...there.map((m) => m.y))];
    const eye = Math.min(most, cam.eye * Math.min(((box.r - box.l) * (1 - 2 * HOME_MARGIN)) / Math.max(1, r - l), ((box.b - box.t) * (1 - 2 * HOME_MARGIN)) / Math.max(1, b - t)));
    // What lay at the middle of the marks now lies at the middle of the box.
    const middle = unproject(size, (l + r) / 2 - bx, (t + b) / 2 - by, false);
    cam = { ...cam, eye, x: cam.x + middle.x / cam.eye, y: cam.y + middle.y / cam.eye };
  }
  return cam;
}

/** A rectangle of the pane, in pixels from its top left. */
export type Box = { l: number; t: number; r: number; b: number };

export type Limits = { least: number; most: number };

/** Out as far as the whole field, which is where it rests; in as far as a drone is large. `worldWidth` is the layout's width, before the plane stretches its depth. */
export function limitsOf(size: Size, plane: Size, worldWidth: number, home: Cam): Limits {
  return { least: Math.min(home.eye, Math.max(LEAST, Math.min(size.width / worldWidth, size.height / (plane.height * COS)) * 0.92)) * 0.98, most: MOST };
}

/** A camera held to the limits and to the plane. */
export const held = (plane: Size, limits: Limits, c: Cam): Cam => ({ ...c, eye: Math.min(limits.most, Math.max(limits.least, c.eye)), x: Math.min(plane.width, Math.max(0, c.x)), y: Math.min(plane.height, Math.max(0, c.y)) });

/** Zoom by a factor toward a screen offset from the middle, keeping the ground under it where it is. None where the limit is reached. */
export function zoomed(size: Size, plane: Size, limits: Limits, c: Cam, factor: number, at: Point): Cam | undefined {
  const eye = Math.min(limits.most, Math.max(limits.least, c.eye * factor));
  if (eye === c.eye) return undefined;
  const under = unproject(size, at.x, at.y, c.tilt);
  const spread = 1 / c.eye - 1 / eye;
  return held(plane, limits, { ...c, eye, x: c.x + under.x * spread, y: c.y + under.y * spread });
}

/** Move the camera so what lay under the middle of the pane now lies a (dx, dy) of the screen away: a scroll, or an arrow key. */
export function panned(size: Size, plane: Size, limits: Limits, c: Cam, dx: number, dy: number): Cam {
  const there = unproject(size, dx, dy, c.tilt);
  return held(plane, limits, { ...c, x: c.x + there.x / c.eye, y: c.y + there.y / c.eye });
}

/** A drag: the ground that was under the pointer where it went down is under it where it is now. `from` is the camera as the drag began. */
export function dragged(size: Size, plane: Size, limits: Limits, from: Cam, down: Point, now: Point): Cam {
  const a = unproject(size, down.x, down.y, from.tilt);
  const b = unproject(size, now.x, now.y, from.tilt);
  return held(plane, limits, { ...from, x: from.x + (a.x - b.x) / from.eye, y: from.y + (a.y - b.y) / from.eye });
}

/** The camera looking at a point of the layout, close and low: lock-on, and `w` going to a call. */
export const lookingAt = (at: Point, eye: number, tilt = true): Cam => ({ x: at.x, y: (at.y - 30) * FLAT, eye, tilt });

/**
 * Where a call's marker stands on the pane's border when the drone is outside the pane, and the angle that points toward it. `at` is
 * the point over the drone where its marker would hang. None where it is inside the pane.
 */
export function edgeOf(size: Size, c: Cam, at: P3): { x: number; y: number; angle: number } | undefined {
  const there = projectOf(size, at, c);
  const sy = there.y - 46;
  const room = { x: size.width / 2 - EDGE_ROOM, y: size.height / 2 - EDGE_ROOM };
  if (Math.abs(there.x) <= room.x && Math.abs(sy) <= room.y) return undefined;
  const k = Math.min(room.x / (Math.abs(there.x) || 1e-6), room.y / (Math.abs(sy) || 1e-6));
  return { x: size.width / 2 + there.x * k, y: size.height / 2 + sy * k, angle: (Math.atan2(sy, there.x) * 180) / Math.PI };
}

/** The marks the field must show at rest: each territory's corners and flag, and every drone with its altitude and the badge it may carry over it. */
export function marksOf(territories: readonly { rect: { x: number; y: number; w: number; h: number }; goal: Point | undefined }[], drones: readonly { at: Point; call: boolean }[], big: number, reach: number): P3[] {
  const marks: P3[] = [];
  for (const one of territories) {
    const { rect } = one;
    for (const x of [rect.x, rect.x + rect.w]) for (const y of [rect.y, rect.y + rect.h + 40]) marks.push({ x, y: y * FLAT, z: 0 });
    if (one.goal !== undefined) marks.push({ x: one.goal.x, y: one.goal.y * FLAT, z: 78 }, { x: one.goal.x + 100, y: one.goal.y * FLAT, z: 40 });
  }
  for (const { at, call } of drones) {
    // A drone holding a call carries its badge over it, a size on the screen and not on the plane: room for it above.
    const over = call ? 150 : 0;
    marks.push({ x: at.x - 38 * big, y: at.y * FLAT, z: reach + 40 * big + over }, { x: at.x + 38 * big, y: at.y * FLAT, z: reach + 40 * big + over }, { x: at.x, y: at.y * FLAT, z: 0 });
  }
  return marks;
}

/** How high a drone's picture reaches above the tallest slab, for the marks above. */
export const reachOf = (foot: number, margin: number, big: number): number => SLAB.high + foot + margin + 18 * big;
