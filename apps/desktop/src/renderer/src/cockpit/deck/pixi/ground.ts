// What lies on the ground under a drone: its shadow, the disc it stands on and the rim round it, a pool of light where it works or calls,
// the rings that say how it stands (a beacon for a call, a stalled amber ring, an accent ring for the one picked on the timeline, dots
// round the one he is locked on). The drone itself hovers still over all of this.
//
// Two layers take it: `shade` is drawn as it is, `glow` is added to what is under it, so a pool of light lights the plot.

import type { Clock } from "./clock";
import { hueOf } from "./drones";
import { PlaneBuilder } from "./plane";
import { fade, mix, type Palette, type Rgba } from "./palette";
import type { DroneSpec, PixiSceneIn } from "./scene";

const TAU = Math.PI * 2;

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

/** How much of a launching drone's ground has arrived: it comes in once the drone has begun to lift. */
export function arrival(spec: DroneSpec, clock: Clock): number {
  if (spec.launch === undefined || clock.pulse <= 0) return 1;
  return Math.min(1, Math.max(0, (clock.since(spec.launch.id, spec.launch.delay) - clock.pulse * 0.7) / (clock.pulse * 0.5)));
}

export function drawGround(shade: PlaneBuilder, glow: PlaneBuilder, s: PixiSceneIn, p: Palette, floor: (tile: string | undefined) => number, clock: Clock): void {
  const ordered = [...s.drones].sort((a, c) => a.at.y - c.at.y);
  for (const spec of ordered) {
    const d = spec.drone;
    const keep = (spec.away ? 0.12 : spec.dim ? 0.4 : 1) * arrival(spec, clock);
    if (keep <= 0) continue;
    const { x: cx, y: cy } = spec.at;
    const z = floor(spec.tile) + 1;
    const hue = hueOf(d.hue, p);
    const r = (d.kind === "job" ? 26 : 20) * 0.85 * s.big;
    const lit = d.hover || d.ping;

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
  }
}

/** A light on either side of a ring: it fades out from the ring to `reach` beyond it and to a third of that inside. */
export function bloom(b: PlaneBuilder, cx: number, cy: number, r: number, c: Rgba, alpha: number, z: number, reach: number): void {
  b.halo(cx, cy, r, r + reach, fade(c, alpha), fade(c, 0), z, 44);
  b.halo(cx, cy, Math.max(0.01, r - reach / 3), r, fade(c, 0), fade(c, alpha), z, 44);
}
