// What lies on the ground under a drone: its shadow, the disc it stands on and the rim round it, a pool of light where it works or calls,
// the rings that say how it stands (a beacon for a call, a stalled amber ring, an accent ring for the one picked on the timeline, dots
// round the one he is locked on), and, for a drone running a tool or a Check, the ground ring with a radar sweep turning on it. The
// drone itself hovers still over all of this; running is this ring and its sweep.
//
// Two layers take it: `shade` is drawn as it is, `glow` is added to what is under it, so a pool and a sweep light the plot.

import type { P3 } from "../layout";
import type { Doing } from "../doing";
import type { Clock } from "./clock";
import { GHOST, hueOf } from "./drones";
import { PlaneBuilder } from "./plane";
import { fade, mix, type Palette, type Rgba } from "./palette";
import type { DroneSpec, PixiSceneIn } from "./scene";
import { turn } from "./clock";

const TAU = Math.PI * 2;
export const RAD = Math.PI / 180;

/** A light that falls off to nothing at `radius`, laid on the ground: rings, each fading into the next, so there is no rim to it. */
export function pool(b: PlaneBuilder, cx: number, cy: number, radius: number, c: Rgba, z: number): void {
  const stops: readonly (readonly [number, number])[] = [[0, 1], [0.3, 0.55], [0.6, 0.2], [1, 0]];
  for (let i = 0; i < stops.length - 1; i += 1) {
    const [[r0, a0], [r1, a1]] = [stops[i]!, stops[i + 1]!];
    b.halo(cx, cy, Math.max(0.01, radius * r0), radius * r1, fade(c, a0), fade(c, a1), z, 36);
  }
}

/** The ring of a sweep, a ring of dashes with a light round it. */
function ring(b: PlaneBuilder, cx: number, cy: number, r: number, half: number, c: Rgba, z: number, opts: { dash?: readonly [number, number] } = {}): void {
  b.stroke(PlaneBuilder.arc(cx, cy, r, 0, TAU, z, 56), half, c, { closed: true, ...opts });
}

/** The colour a working drone's action is drawn in: a Check amber while it runs, green once it has passed, red while it fails. */
export function actOf(drone: DroneSpec["drone"], p: Palette): Rgba {
  const doing: Doing | undefined = drone.doing;
  if (doing === "write") return p.hot;
  if (doing === "run") return drone.check === "bad" ? p.bad : drone.check === "ok" ? p.good : p.ask;
  return p.run;
}

/** How much of a launching drone's ground has arrived: it comes in once the drone has begun to lift. A docking one's goes as it flies back. */
export function arrival(spec: DroneSpec, clock: Clock): number {
  if (spec.dock !== undefined) return clock.pulse <= 0 ? 0 : 1 - Math.min(1, Math.max(0, clock.since(spec.dock.id, spec.dock.delay) / (clock.pulse * 0.8)));
  if (spec.launch === undefined || clock.pulse <= 0) return 1;
  return Math.min(1, Math.max(0, (clock.since(spec.launch.id, spec.launch.delay) - clock.pulse * 0.7) / (clock.pulse * 0.5)));
}

/** A radar sweep: a wedge of light behind a bright leading edge, turning clockwise on its ring. */
export function sweep(b: PlaneBuilder, cx: number, cy: number, r: number, lead: number, c: Rgba, z: number, degrees = 70): void {
  const slices = 14;
  const span = degrees * RAD;
  const step = span / slices;
  const at = (angle: number): Rgba => fade(c, 0.55 * Math.pow(Math.max(0, 1 - (lead - angle) / span), 1.4));
  for (let k = 0; k < slices; k += 1) {
    const [a1, a0] = [lead - k * step, lead - (k + 1) * step];
    const points: P3[] = [{ x: cx, y: cy, z }, { x: cx + Math.cos(a0) * r, y: cy + Math.sin(a0) * r, z }, { x: cx + Math.cos(a1) * r, y: cy + Math.sin(a1) * r, z }];
    const [c0, c1] = [at(a0), at(a1)];
    b.shaded(points, [fade(c1, 0.3), c0, c1]);
  }
  b.stroke([{ x: cx, y: cy, z }, { x: cx + Math.cos(lead) * r, y: cy + Math.sin(lead) * r, z }], 0.7, fade(c, 0.75));
}

export function drawGround(shade: PlaneBuilder, glow: PlaneBuilder, s: PixiSceneIn, p: Palette, floor: (tile: string | undefined) => number, clock: Clock): void {
  const ordered = [...s.drones].sort((a, c) => a.at.y - c.at.y);
  for (const spec of ordered) {
    const d = spec.drone;
    const keep = (spec.away ? 0.12 : spec.ghost ? GHOST : spec.dim ? 0.4 : 1) * arrival(spec, clock);
    if (keep <= 0) continue;
    const { x: cx, y: cy } = spec.at;
    const z = floor(spec.tile) + 1;
    const hue = hueOf(d.hue, p);
    const r = 20 * 0.85 * s.big;
    const act = d.doing === undefined ? undefined : actOf(d, p);
    const lit = (d.hover || d.ping) && !spec.ghost;

    // The pool of light: the hue of the drone, or the colour of what it is doing over the plot.
    if (lit) pool(glow, cx, cy, r * 2.3, fade(d.doing === "read" ? p.run : d.doing === "write" ? p.hot : hue, 0.34 * keep), z);
    // A beacon: rings going out from a drone that holds a call. Held at one radius, as a call is held until he opens it.
    if (d.ping) {
      const wide = spec.call === undefined ? 1.7 : 1.9;
      ring(shade, cx, cy, r * wide, spec.call === undefined ? 0.8 : 1.2, fade(hue, 0.6 * keep), z);
      bloom(glow, cx, cy, r * wide, hue, 0.35 * keep, z, r * 0.5);
    }
    // Stuck: an amber ring round it, its rotors stalled.
    if (d.stuck) {
      ring(shade, cx, cy, r * 1.5, 1, fade(p.ask, 0.8 * keep), z);
      bloom(glow, cx, cy, r * 1.5, p.ask, 0.4 * keep, z, r * 0.45);
    }
    // The drone a picked event is about.
    if (spec.focused) {
      ring(shade, cx, cy, r * 1.8, 1.1, fade(p.accent, keep), z);
      bloom(glow, cx, cy, r * 1.8, p.accent, 0.5 * keep, z, r * 0.6);
      ring(shade, cx, cy, r * 2.6, 0.7, fade(p.accent, 0.5 * keep), z);
    }
    // Its shadow is tight and dark when it stands low, wide and soft when it flies high.
    pool(shade, cx, cy, r * (d.hover ? 1.3 : 0.8) * 0.93, fade(p.base, (d.hover ? 0.5 : 0.95) * keep), z + 0.05);
    shade.disc(cx, cy, r, fade(p.fg, 0.09 * keep), z + 0.1);
    ring(shade, cx, cy, r, spec.selected ? 1 : 0.7, fade(spec.selected ? p.accent : spec.hot ? p.fg : mix(p.muted, hue, 0.7), (spec.selected || spec.hot ? 1 : 0.7) * keep), z + 0.15);
    if (spec.selected) ring(shade, cx, cy, r * 1.9, 1.2, fade(p.accent, 0.9 * keep), z + 0.15, { dash: [1, 4] });

    // Running a tool or a Check: its ring on the ground, and a sweep turning on it while it is one of the few that move.
    if (d.doing === "run" && spec.aim !== undefined && act !== undefined) {
      const rz = z + 0.5;
      const lead = spec.loop && clock.pulse > 0 ? -Math.PI / 2 + turn(clock.now, 14000) * TAU : -Math.PI / 4;
      shade.disc(cx, cy, s.orbit, fade(act, 0.05 * keep), rz);
      ring(shade, cx, cy, s.orbit, 0.75, fade(act, 0.75 * keep), rz, { dash: [3.5, 4] });
      bloom(glow, cx, cy, s.orbit, act, 0.34 * keep, rz, 10);
      sweep(glow, cx, cy, s.orbit, lead, fade(act, keep), rz);
    }
  }
}

/** A light on either side of a ring: it fades out from the ring to `reach` beyond it and to a third of that inside. */
export function bloom(b: PlaneBuilder, cx: number, cy: number, r: number, c: Rgba, alpha: number, z: number, reach: number): void {
  b.halo(cx, cy, r, r + reach, fade(c, alpha), fade(c, 0), z, 44);
  b.halo(cx, cy, Math.max(0.01, r - reach / 3), r, fade(c, 0), fade(c, alpha), z, 44);
}
