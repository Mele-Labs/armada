// The activity log as motion on the ground and in the air between: what an event looks like when it is new. A Check turns a radar cone on
// the drone that runs it; something said sends a ring out from its drone; a plot that was touched sends one out from its middle; a merge flashes the objective, sends a
// shockwave over its territory and a column of light up the flag. A launch and a pull request's hop are the drones' and the crates' own
// (`fliers.ts`, `stands.ts`), and the plots that rise are the stage's. Each is a function of how long ago it began, so none keeps state.
// Light, so it is added to what is under it.

import { onPlane } from "../bf-parts";
import type { Play } from "../plays";
import type { Clock } from "./clock";
import { pool, sweep } from "./ground";
import { PlaneBuilder } from "./plane";
import { fade, type Palette } from "./palette";
import type { DroneSpec, PixiSceneIn } from "./scene";

const TAU = Math.PI * 2;

/** The play's place in a run of `ms`, 0 to 1, or nothing where it has not begun or is over. */
const within = (since: number, ms: number): number | undefined => (since < 0 || since >= ms ? undefined : since / ms);

/** One ring going out: the radius it grows to, and how it fades. */
function ring(b: PlaneBuilder, at: { x: number; y: number }, z: number, reach: number, t: number, from: number, c: Palette["fg"], half: number): void {
  const r = reach * (from + (1.7 - from) * t);
  b.stroke(PlaneBuilder.arc(at.x, at.y, r, 0, TAU, z, 48), half, fade(c, 0.85 * (1 - t)), { closed: true });
}

export function drawPlays(glow: PlaneBuilder, s: PixiSceneIn, p: Palette, floor: (tile: string | undefined) => number, clock: Clock): void {
  if (clock.pulse <= 0 || s.plays.length === 0) return;
  const pulse = clock.pulse;
  const byKey = new Map<string, DroneSpec>(s.drones.map((one) => [one.drone.key, one] as const));
  for (const play of s.plays) {
    const since = clock.since(play.id, play.delay);
    const drone = play.key === undefined ? undefined : byKey.get(play.key);
    if (play.kind === "capture") {
      capture(glow, s, p, play, since, pulse);
      continue;
    }
    if (drone === undefined) continue;
    const z = floor(drone.tile) + 1.5;
    if (play.kind === "sweep") {
      const t = within(since, pulse * 2.1);
      if (t !== undefined) {
        // It comes in, turns twice and goes out.
        const hold = Math.min(1, t / 0.1, (1 - t) / 0.12);
        sweep(glow, drone.at.x, drone.at.y, 110, -Math.PI / 2 + t * 4 * Math.PI, fade(p.fg, hold), z, 62);
      }
    }
    if (play.kind === "blip" || play.kind === "tracer") {
      // Two rings, one after the other.
      const lap = pulse * 0.9;
      const t = within(since, lap * 2);
      if (t !== undefined) ring(glow, drone.at, z, 80, (t * 2) % 1, 0.15, p.fg, 0.75);
    }
    if (play.kind === "launch" || play.kind === "sweep" || play.kind === "tracer") {
      const main = drone.touches[0];
      const t = main === undefined ? undefined : within(since - (play.kind === "launch" ? 1000 : 300), pulse * 1.3);
      if (main !== undefined && t !== undefined) ring(glow, main.at, floor(main.tile) + 1.5, 100, t, 0.15, p.accent, 1);
    }
  }
}

/** A merge: a flash on the objective, a shockwave out over its territory, then a column of light up the flag. */
function capture(glow: PlaneBuilder, s: PixiSceneIn, p: Palette, play: Play, since: number, pulse: number): void {
  const root = play.root ?? s.crates.find((one) => one.pr.number === play.number)?.pr.root;
  const territory = root === undefined ? undefined : s.layout.territories.find((one) => one.root === root);
  if (territory?.goal === undefined) return;
  const at = onPlane(territory.goal);
  const z = 1;
  const wait = 900;
  const flash = within(since - wait, pulse * 0.9);
  if (flash !== undefined) pool(glow, at.x, at.y, 44 * (0.4 + 1.5 * flash), fade(p.fg, Math.min(1, flash / 0.14) * (1 - Math.max(0, (flash - 0.14) / 0.86)) * 0.9), z);
  const quake = within(since - wait, pulse * 1.7);
  if (quake !== undefined) ring(glow, at, z, 260, quake, 0.08, p.fg, 1.4);
  const late = within(since - wait - 350, pulse * 1.7);
  if (late !== undefined) ring(glow, at, z, 260, late, 0.08, p.fg, 0.6);
  const column = within(since - wait, pulse * 0.9);
  if (column !== undefined) {
    const up = Math.min(1, column / 0.25);
    const live = column < 0.25 ? column / 0.25 : 1 - (column - 0.25) / 0.75;
    const top = 170 * up;
    const line = [{ x: at.x, y: at.y, z }, { x: at.x, y: at.y, z: z + Math.max(1, top) }];
    glow.stroke(line, 4, fade(p.fg, 0.8 * 0.6 * live), { normal: { x: 1, y: 0 }, shade: (q) => fade(p.fg, 0.48 * live * (1 - (q.z - z) / Math.max(1, top))) });
    glow.stroke(line, 9, fade(p.fg, 0.1 * live), { normal: { x: 1, y: 0 }, shade: (q) => fade(p.fg, 0.1 * live * (1 - (q.z - z) / Math.max(1, top))) });
  }
}
