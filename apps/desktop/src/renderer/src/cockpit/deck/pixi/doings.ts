// What a working drone is doing over the plot it aims at. Reading: a narrow cone sweeps the plot from the drone, a grid lies on it and a
// light scan line crosses it, a ring goes out from the grid. Writing: a laser fires down in bursts, on for 1.2 seconds and off for 0.8, with
// a hot spot where it lands and a few sparks. Running is the ring and sweep the drone's ground carries (`ground.ts`). Only the drones in
// loopsOf move; the rest show the same form still. All of it is light, so it is added to what is under it.
//
// The grid, the spot and the sparks lie on the plot and are plane geometry; the laser and the cone are lines through the air between the
// drone and the plot, drawn in screen space on one graphic each time the camera or the clock moves them.

import { Graphics } from "pixi.js";
import type { Container } from "pixi.js";

import { soft, there, turn, type Clock } from "./clock";
import { actOf, pool } from "./ground";
import { PlaneBuilder } from "./plane";
import { fade, hex, mix, type Palette, type Rgba } from "./palette";
import { project, type View } from "./project";
import { OUTPOST_UP, type DroneSpec, type PixiSceneIn } from "./scene";
import { drawThinking } from "./off-floor";

const TAU = Math.PI * 2;

/** The sparks the impact throws: where each goes, in plane units. */
const SPARKS = [[-16, -9], [15, -11], [-11, 8], [17, 7]] as const;

/** A laser is on for the first 1.2 seconds of two, as it fires in bursts. */
const burst = (now: number): boolean => turn(now, 2000) < 0.6;

const doers = (s: PixiSceneIn): DroneSpec[] => s.drones.filter((one) => one.aim !== undefined && (one.drone.doing === "read" || one.drone.doing === "write"));

/** What lies on the plot: the grid and its scan and ring for a read, the hot spot and its sparks for a write. */
export function drawDoings(glow: PlaneBuilder, s: PixiSceneIn, p: Palette, floor: (tile: string | undefined) => number, clock: Clock): void {
  for (const spec of doers(s)) {
    const { aim, drone } = spec;
    if (aim === undefined) continue;
    const act = actOf(drone, p);
    const moving = spec.loop && clock.pulse > 0;
    const { x, y } = aim.at;
    if (drone.doing === "read") {
      const z = floor(aim.tile) + 1.5;
      const [x0, y0] = [x - aim.wide / 2, y - aim.deep / 2];
      const line = fade(act, 0.2);
      for (let g = 9; g < aim.wide; g += 9) glow.stroke([{ x: x0 + g, y: y0, z }, { x: x0 + g, y: y0 + aim.deep, z }], 0.4, line);
      for (let g = 9; g < aim.deep; g += 9) glow.stroke([{ x: x0, y: y0 + g, z }, { x: x0 + aim.wide, y: y0 + g, z }], 0.4, line);
      glow.stroke(PlaneBuilder.rounded(x0, y0, aim.wide, aim.deep, 2, z), 0.6, fade(act, 0.4), { closed: true });
      // The scan line crosses the grid and back; held at the middle where it does not move.
      const at = x0 + (moving ? soft(there(turn(clock.now, 4800))) : 0.5) * (aim.wide - 2);
      const edge = (a: number): Rgba => fade(act, a);
      for (const [from, to, near, far] of [[at - 7, at, 0, 0.38], [at, at + 2, 0.38, 0.38], [at + 2, at + 9, 0.38, 0]] as const) {
        glow.shaded([{ x: from, y: y0, z }, { x: to, y: y0, z }, { x: to, y: y0 + aim.deep, z }, { x: from, y: y0 + aim.deep, z }], [edge(near), edge(far), edge(far), edge(near)]);
      }
      glow.stroke([{ x: at + 1, y: y0, z }, { x: at + 1, y: y0 + aim.deep, z }], 1, fade(mix(act, p.fg, 0.35), 1));
      // The ring that goes out from it: held at the size it starts at where it does not move.
      const t = moving ? turn(clock.now, 1200) : 0;
      const out = moving ? 0.3 + 1.4 * t : 1;
      glow.stroke(PlaneBuilder.arc(x, y, 13 * out, 0, TAU, z, 36), 0.75, fade(act, moving ? 0.85 * (1 - t) : 0.75), { closed: true });
    } else {
      const z = floor(aim.tile) + 8.5;
      if (!moving || burst(clock.now)) pool(glow, x, y, 18, fade(mix(act, p.fg, 0.35), 0.8), z);
      if (!moving) continue;
      SPARKS.forEach(([sx, sy], i) => {
        const t = turn(clock.now - i * 140, 2000) / 0.26;
        if (t >= 1) return;
        glow.flat(x + sx * t - 2, y + sy * t - 2, 4, 4, z, fade(act, 1 - t));
      });
    }
  }
}

/** The lines through the air: the laser a writing drone fires and the cone a reading one sweeps, on one graphic over everything. */
export class Beams {
  private readonly g = new Graphics();

  constructor(layer: Container) {
    this.g.blendMode = "add";
    this.g.eventMode = "none";
    layer.addChild(this.g);
  }

  update(s: PixiSceneIn, p: Palette, view: View, floor: (tile: string | undefined) => number, clock: Clock): void {
    const g = this.g;
    g.clear();
    if (view.width === 0) return;
    for (const spec of doers(s)) {
      const { aim, drone } = spec;
      if (aim === undefined) continue;
      const moving = spec.loop && clock.pulse > 0;
      const top = floor(aim.tile);
      const body = project(view, { x: spec.at.x, y: spec.at.y, z: spec.alt });
      // From the lens, which hangs under the middle of the drone.
      const from = { x: body.x, y: body.y + 7.6 * s.big * view.eye * body.scale };
      if (drone.doing === "write") {
        if (moving && !burst(clock.now)) continue;
        const hit = project(view, { x: aim.at.x, y: aim.at.y, z: top + 8 });
        const u = view.eye * ((body.scale + hit.scale) / 2);
        const flame = p.hot;
        const core = mix(flame, p.fg, 0.6);
        for (const [width, alpha] of [[11, 0.1], [7, 0.22], [3.5, 0.34]] as const) g.moveTo(from.x, from.y).lineTo(hit.x, hit.y).stroke({ color: hex(flame), alpha, width: width * u, cap: "round" });
        g.moveTo(from.x, from.y).lineTo(hit.x, hit.y).stroke({ color: hex(core), alpha: 0.95, width: 1.6 * u, cap: "round" });
        for (const [r, alpha] of [[16, 0.12], [10, 0.22], [5, 0.4]] as const) g.circle(hit.x, hit.y, r * u).fill({ color: hex(mix(flame, p.fg, 0.35)), alpha });
        for (const [r, alpha] of [[8, 0.18], [4, 0.35]] as const) g.circle(from.x, from.y, r * u).fill({ color: hex(flame), alpha });
      } else {
        // A narrow cone from the drone to the plot, turning about the drone so that its foot sweeps the plot's width.
        const hit = project(view, { x: aim.at.x, y: aim.at.y, z: top });
        const reach = project(view, { x: aim.at.x + aim.wide / 2, y: aim.at.y, z: top });
        const [dx, dy] = [hit.x - from.x, hit.y - from.y];
        const length = Math.hypot(dx, dy) || 1;
        const sweep = Math.min((26 * Math.PI) / 180, Math.atan(Math.abs(reach.x - hit.x) / length));
        const turned = moving ? (soft(there(turn(clock.now, 4800))) * 2 - 1) * sweep : 0;
        const [cos, sin] = [Math.cos(turned), Math.sin(turned)];
        const [rx, ry] = [dx * cos - dy * sin, dx * sin + dy * cos];
        const u = view.eye * ((body.scale + hit.scale) / 2);
        const [nx, ny] = [(-ry / length) * 6 * u, (rx / length) * 6 * u];
        const tone = hex(p.run);
        const cone = [from.x, from.y, from.x + rx + nx, from.y + ry + ny, from.x + rx - nx, from.y + ry - ny];
        g.poly(cone).fill({ color: tone, alpha: 0.2 }).stroke({ color: tone, alpha: 0.6, width: 1, join: "round" });
        g.circle(from.x, from.y, 5 * u).fill({ color: tone, alpha: 0.3 });
      }
    }
    this.calls(s, p, view);
    drawThinking(g, s, p, view, clock);
  }

  /** The line from each drone calling a service to that service's outpost: the laser's layers, held steady, since a call is one thing and not a burst. */
  private calls(s: PixiSceneIn, p: Palette, view: View): void {
    const g = this.g;
    for (const spec of s.drones) {
      const outpost = spec.calling === undefined || spec.away ? undefined : s.outposts.find((one) => one.service === spec.calling);
      if (outpost === undefined) continue;
      const body = project(view, { x: spec.at.x, y: spec.at.y, z: spec.alt });
      const from = { x: body.x, y: body.y + 7.6 * s.big * view.eye * body.scale };
      const foot = project(view, { x: outpost.at.x, y: outpost.at.y, z: 0 });
      const to = { x: foot.x, y: foot.y - OUTPOST_UP * view.eye * foot.scale };
      const u = view.eye * ((body.scale + foot.scale) / 2);
      for (const [width, alpha] of [[11, 0.1], [7, 0.22], [3.5, 0.34]] as const) g.moveTo(from.x, from.y).lineTo(to.x, to.y).stroke({ color: hex(p.run), alpha, width: width * u, cap: "round" });
      g.moveTo(from.x, from.y).lineTo(to.x, to.y).stroke({ color: hex(mix(p.run, p.fg, 0.6)), alpha: 0.95, width: 1.6 * u, cap: "round" });
    }
  }

  destroy(): void {
    this.g.destroy();
  }
}
