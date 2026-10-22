// What lies over the plots and changes with each event, apart from the plots so that an event never draws the floor again: the files'
// bars, the glow a Job's reads and writes leave on them, the outline of the plots a Job has touched, and its route while the Field is
// scrubbed back. Each follows its plot's height as it eases.

import { FLAT, type Bar, type P3 } from "../layout";
import { pool } from "./ground";
import { PlaneBuilder } from "./plane";
import { fade, mix, type Palette, type Rgba } from "./palette";
import type { PixiSceneIn } from "./scene";

/** How far over its plot a file's bar floats, and the route over that. */
const LIFT = 10;
const ROUTE_UP = 14;
/** The least a glow's pool reaches round its file, in the layout's units. It reaches more than half across its plot, since a bar is small and a drone over it covers its middle. */
const POOL = 60;

const BAR_TONE: Record<NonNullable<Bar["act"]>, (p: Palette) => Rgba> = { write: (p) => p.good, edit: (p) => p.accent, remove: (p) => p.bad };

/** A file: a small bar floating over its plot with a shadow on it, lit in the colour of what is being done to it. */
function drawBar(b: PlaneBuilder, bar: Bar, height: number, p: Palette): void {
  const { rect } = bar;
  const [x, y, w, h] = [rect.x, rect.y * FLAT, rect.w, rect.h * FLAT];
  const lift = height + LIFT;
  const tone = bar.act === undefined ? mix(p.base, p.fg, 0.3) : BAR_TONE[bar.act](p);
  b.flat(x + 1, y + 2.5, w, h, height + 0.2, [0, 0, 0, 0.35]);
  if (bar.act !== undefined) b.halo(x + w / 2, y + h / 2, Math.max(w, h) * 0.5, Math.max(w, h) * 0.5 + 7, fade(tone, 0.4), fade(tone, 0), lift - 0.1, 16);
  b.flat(x, y, w, h, lift, bar.act === "remove" ? fade(tone, 0.6) : tone);
}

/**
 * The glow of a file: its bar tinted, and a pool of light on its plot round it that reads from across the field, in the hue of a write,
 * or the cool hue of a read and fainter, as strong as what is left of it.
 */
function drawGlow(b: PlaneBuilder, glow: PlaneBuilder, bar: Bar, height: number, act: "read" | "write", v: number, p: Palette): void {
  const { rect } = bar;
  const [x, y, w, h] = [rect.x, rect.y * FLAT, rect.w, rect.h * FLAT];
  const tone = act === "write" ? p.hot : p.look;
  const write = act === "write";
  const reach = Math.max(POOL, Math.min(bar.tile.rect.w, bar.tile.rect.h * FLAT) * 0.6);
  pool(glow, x + w / 2, y + h / 2, reach, fade(tone, (write ? 0.8 : 0.6) * v), height + 0.3);
  b.flat(x, y, w, h, height + LIFT + 0.05, fade(tone, (write ? 0.85 : 0.55) * v));
}

export function drawMarks(b: PlaneBuilder, glow: PlaneBuilder, s: PixiSceneIn, p: Palette, heights: ReadonlyMap<string, number>): void {
  const tiles = new Map(s.layout.territories.flatMap((one) => one.tiles.map((tile) => [tile.path, tile] as const)));
  // The plots the Job has touched: an accent line just inside the edge, over the plan's own frame.
  for (const path of s.touched ?? []) {
    const tile = tiles.get(path);
    if (tile === undefined) continue;
    const z = (heights.get(path) ?? 0) + 0.08;
    const { rect } = tile;
    const [x0, x1, y0, y1] = [rect.x + 0.6, rect.x + rect.w - 0.6, rect.y * FLAT + 0.6, (rect.y + rect.h) * FLAT - 0.6];
    if (x1 <= x0 || y1 <= y0) continue;
    const frame = [{ x: x0, y: y0, z }, { x: x1, y: y0, z }, { x: x1, y: y1, z }, { x: x0, y: y1, z }];
    b.stroke(frame, 0.8, fade(p.accent, 0.9), { closed: true });
    glow.stroke(frame, 2.4, fade(p.accent, 0.14), { closed: true });
  }
  const bars = [...s.layout.bars.values()].sort((a, c) => a.rect.y - c.rect.y || a.rect.x - c.rect.x);
  for (const bar of bars) {
    const height = heights.get(bar.tile.path) ?? 0;
    drawBar(b, bar, height, p);
    const lit = s.glow?.get(bar.path);
    if (lit !== undefined) drawGlow(b, glow, bar, height, lit.act, lit.v, p);
  }
  // The route: a line from each stop to the next, dotted into a read and dashed into a write. Its numbers stand over it (`stands.ts`).
  const route = s.route ?? [];
  const up = (spot: (typeof route)[number]): P3 => ({ ...spot.at, z: (heights.get(spot.tile) ?? 0) + ROUTE_UP });
  for (let k = 1; k < route.length; k += 1) {
    const [from, to] = [route[k - 1]!, route[k]!];
    if (from.at.x === to.at.x && from.at.y === to.at.y) continue;
    b.stroke([up(from), up(to)], 0.7, fade(p.fg, 0.75), { dash: to.act === "read" ? [1, 3] : [5, 3] });
  }
  for (const spot of route) b.stroke([{ ...spot.at, z: (heights.get(spot.tile) ?? 0) + LIFT }, up(spot)], 0.45, fade(p.fg, 0.4));
}
