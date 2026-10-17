// The map's projection on the canvas: `camera.ts`'s own turn, scale and perspective, with the pitch free to be anywhere between the wide
// and the near one so the canvas can ease between them as the CSS did. The shader does this for every vertex of the ground; this file
// does it for the few things that stand upright and are placed from script (a flag, a name), and packs the numbers the shader
// reads. At a pitch of `CAM.wide` or `CAM.near` it is `projectOf`, point for point.

import { CAM, type P3, type Size } from "../layout";
import type { Cam } from "../camera";

/** The camera as the canvas draws it this frame: where it looks, how close, the pitch in degrees, and the pane. */
export type View = { x: number; y: number; eye: number; pitch: number; width: number; height: number };

const rad = (deg: number) => (deg * Math.PI) / 180;

export const pitchOf = (cam: Pick<Cam, "tilt">): number => (cam.tilt ? CAM.near : CAM.wide);

export const viewOf = (cam: Cam, size: Size, pitch = pitchOf(cam)): View => ({ x: cam.x, y: cam.y, eye: cam.eye, pitch, width: size.width, height: size.height });

/** Where a point of the scene lands on the canvas, in pixels from its top left, and how much the perspective scales what stands there. */
export function project(v: View, p: P3): { x: number; y: number; scale: number } {
  const phi = rad(v.pitch);
  const yaw = rad(CAM.yaw);
  const [dx, dy, z] = [(p.x - v.x) * v.eye, (p.y - v.y) * v.eye, p.z * v.eye];
  const ux = dx * Math.cos(yaw) - dy * Math.sin(yaw);
  const uy = dx * Math.sin(yaw) + dy * Math.cos(yaw);
  const depth = v.width * 2.8;
  const scale = depth / (depth - (uy * Math.sin(phi) + z * Math.cos(phi)));
  return { x: v.width / 2 + ux * scale, y: v.height / 2 + (uy * Math.cos(phi) - z * Math.sin(phi)) * scale, scale };
}

/** The point of the plane, level at height `z`, that lands at a pixel: `project` run the other way, which a level surface allows. */
export function unproject(v: View, x: number, y: number, z: number): { x: number; y: number } {
  const [phi, yaw] = [rad(v.pitch), rad(CAM.yaw)];
  const depth = v.width * 2.8;
  const [sinP, cosP] = [Math.sin(phi), Math.cos(phi)];
  const [a, b, h] = [x - v.width / 2, y - v.height / 2, z * v.eye];
  // The screen's height gives the depth along the camera's ground, and its width then scales to the breadth.
  const uy = (h * sinP + (b * (depth - h * cosP)) / depth) / (cosP + (b * sinP) / depth);
  const scale = depth / (depth - (uy * sinP + h * cosP));
  const ux = a / scale;
  const [dx, dy] = [ux * Math.cos(yaw) + uy * Math.sin(yaw), -ux * Math.sin(yaw) + uy * Math.cos(yaw)];
  return { x: v.x + dx / v.eye, y: v.y + dy / v.eye };
}

/** The numbers the vertex shader reads, in the order its uniforms are declared. */
export function packView(v: View, into: { cam: Float32Array; trig: Float32Array; screen: Float32Array }): void {
  const [yaw, phi] = [rad(CAM.yaw), rad(v.pitch)];
  into.cam[0] = v.x;
  into.cam[1] = v.y;
  into.trig[0] = Math.cos(yaw);
  into.trig[1] = Math.sin(yaw);
  into.trig[2] = Math.cos(phi);
  into.trig[3] = Math.sin(phi);
  into.screen[0] = v.width / 2;
  into.screen[1] = v.height / 2;
  into.screen[2] = v.width * 2.8;
}

/** The camera's ease, `cubic-bezier(0.22, 0.8, 0.24, 1)` as the stylesheet writes it, solved for the progress at time `t` of the way through. */
export function easeOut(t: number): number {
  if (t <= 0) return 0;
  if (t >= 1) return 1;
  const [x1, y1, x2, y2] = [0.22, 0.8, 0.24, 1];
  const at = (a: number, b: number, s: number) => 3 * a * s * (1 - s) * (1 - s) + 3 * b * s * s * (1 - s) + s * s * s;
  let [lo, hi] = [0, 1];
  for (let pass = 0; pass < 24; pass += 1) {
    const mid = (lo + hi) / 2;
    if (at(x1, x2, mid) < t) lo = mid;
    else hi = mid;
  }
  return at(y1, y2, (lo + hi) / 2);
}

export const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
