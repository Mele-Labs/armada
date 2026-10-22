// What a Job's Field draws off the code floor: its pad at the near edge with a light for each check, the red line from a failing check
// to the file Fleet read from its log, the faint line from a subagent to the drone that sent it, and the dots circling a drone that thinks.
// A light pulses and the dots turn only while their drone is one of the few the map animates (`loopsOf`), and stand still under
// reduced motion. `docs/concepts/cockpit.md`, *A Job open*.

import type { Graphics } from "pixi.js";

import { FLAT, type Bar } from "../layout";
import { turn, type Clock } from "./clock";
import { arrival } from "./ground";
import { PlaneBuilder } from "./plane";
import { fade, hex, mix, type Palette, type Rgba } from "./palette";
import { project, type View } from "./project";
import type { DroneSpec, PadSpec, PixiSceneIn } from "./scene";

const TAU = Math.PI * 2;

/** How far over its plot a file's bar floats (`marks.ts`), which the blame line ends at. */
const BAR_LIFT = 10;

const LIGHT: Record<PadSpec["lights"][number]["state"], (p: Palette) => Rgba> = { idle: (p) => p.dormant, run: (p) => p.run, ok: (p) => p.good, bad: (p) => p.bad };

/** Whether the pad's lights move: a drone is on the pad running checks, and it is one of those that loop. */
export const padMoving = (s: PixiSceneIn, clock: Clock): boolean => clock.pulse > 0 && s.drones.some((one) => one.drone.off === "pad" && one.loop);

/** The pad: a slab on the ground in the accent, and a light along its front edge for each check, a running one pulsing in the in-flight hue. */
export function drawPad(shade: PlaneBuilder, glow: PlaneBuilder, s: PixiSceneIn, p: Palette, clock: Clock): void {
  const pad = s.pad;
  if (pad === undefined) return;
  const [x, y] = [pad.at.x - pad.w / 2, pad.at.y - pad.d / 2];
  shade.fan(PlaneBuilder.rounded(x, y, pad.w, pad.d, 4, 0.4), mix(p.base, p.accent, 0.1));
  shade.stroke(PlaneBuilder.rounded(x, y, pad.w, pad.d, 4, 0.45), 0.6, fade(p.accent, 0.5), { closed: true });
  const moving = padMoving(s, clock);
  for (const light of pad.lights) {
    const tone = LIGHT[light.state](p);
    const lit = light.state !== "idle";
    // A running light breathes over one pulse; held at its brightest where nothing moves.
    const breath = light.state === "run" && moving ? 0.35 + 0.65 * (0.5 + 0.5 * Math.cos(turn(clock.now, clock.pulse) * TAU)) : 1;
    shade.disc(light.at.x, light.at.y, 3.2, fade(lit ? tone : mix(p.base, p.fg, 0.2), lit ? breath : 1), 0.6);
    if (lit) glow.halo(light.at.x, light.at.y, 3, 11, fade(tone, 0.55 * breath), fade(tone, 0), 0.65, 24);
  }
}

/** From each failing light to the file Fleet read from its log: a dashed red line up to the file's bar, and the bar outlined in red. */
export function drawBlame(b: PlaneBuilder, glow: PlaneBuilder, s: PixiSceneIn, p: Palette, heights: ReadonlyMap<string, number>): void {
  for (const one of s.pad?.blame ?? []) {
    const bar: Bar | undefined = s.layout.bars.get(one.bar);
    if (bar === undefined) continue;
    const z = (heights.get(bar.tile.path) ?? 0) + BAR_LIFT + 0.1;
    const { rect } = bar;
    const [x0, y0, x1, y1] = [rect.x - 1.5, rect.y * FLAT - 1.5, rect.x + rect.w + 1.5, (rect.y + rect.h) * FLAT + 1.5];
    const line = [{ ...one.from, z: 0.7 }, { x: bar.cx, y: bar.cy * FLAT, z }];
    b.stroke(line, 0.7, fade(p.bad, 0.95), { dash: [4, 3] });
    glow.stroke(line, 2.4, fade(p.bad, 0.18));
    const frame = [{ x: x0, y: y0, z }, { x: x1, y: y0, z }, { x: x1, y: y1, z }, { x: x0, y: y1, z }];
    b.stroke(frame, 0.8, p.bad, { closed: true });
    glow.stroke(frame, 2.6, fade(p.bad, 0.2), { closed: true });
  }
}

/** A faint line from each subagent to the drone that sent it, coming in as it lifts off and going as it flies back. */
export function drawLinks(b: PlaneBuilder, s: PixiSceneIn, p: Palette, clock: Clock): void {
  for (const spec of s.drones) {
    if (spec.link === undefined || spec.ghost) continue;
    const keep = arrival(spec, clock);
    if (keep > 0) b.stroke([spec.link, spec.flying], 0.45, fade(p.muted, 0.4 * keep), { dash: [2, 3] });
  }
}

const thinking = (s: PixiSceneIn): DroneSpec[] => s.drones.filter((one) => one.drone.off === "think" && !one.ghost && !one.away);

/** Whether a thinking drone's dots turn. */
export const thinkMoving = (s: PixiSceneIn, clock: Clock): boolean => clock.pulse > 0 && thinking(s).some((one) => one.loop);

/** Three dots circling a thinking drone slowly, the near one brightest. Still, they stand where a turn would start. */
export function drawThinking(g: Graphics, s: PixiSceneIn, p: Palette, view: View, clock: Clock): void {
  for (const spec of thinking(s)) {
    const body = project(view, { x: spec.at.x, y: spec.at.y, z: spec.alt });
    const u = view.eye * body.scale * s.big;
    const start = spec.loop && clock.pulse > 0 ? turn(clock.now, clock.pulse * 3) * TAU : 0.4;
    for (let k = 0; k < 3; k += 1) {
      const a = start + (k * TAU) / 3;
      const near = 0.5 + 0.5 * Math.sin(a);
      g.circle(body.x + Math.cos(a) * 24 * u, body.y - 4 * u + Math.sin(a) * 9 * u, 2.6 * u).fill({ color: hex(p.accent), alpha: 0.4 + 0.6 * near });
    }
  }
}
