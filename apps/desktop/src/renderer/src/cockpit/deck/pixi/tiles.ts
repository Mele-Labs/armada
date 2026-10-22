// The plots and what stands on them: each directory a slab with its height set by what works over it, the files as bars lifted a little
// above their slab, and what has been built as a ring lying on the ground under its drone. These follow the heights as they ease, so
// they are rebuilt each frame the heights move; the ground does not.

import { onPlane } from "../bf-parts";
import { arcsOf, bandOf, roadOf } from "../build";
import { FLAT, rand, type P3, type Tile } from "../layout";
import { pillRect } from "../pills";
import { PlaneBuilder } from "./plane";
import { fade, mix, onGround, type Palette, type Rgba } from "./palette";
import type { PixiSceneIn, Placed, PlotPiece } from "./scene";

/**
 * What the plots are drawn from, and nothing else: the stage draws them again only when one of these changes, so a new event, a glow or
 * a bar moving leaves the floor as it is. The files' bars are `marks.ts`.
 */
export type TilesIn = Pick<PixiSceneIn, "big" | "lit" | "clashing" | "hotTile" | "planned" | "builds"> & { layout: Pick<PixiSceneIn["layout"], "territories"> };

/** The part of a scene the plots read. */
export const tilesIn = (s: PixiSceneIn): TilesIn => ({ layout: { territories: s.layout.territories }, big: s.big, lit: s.lit, clashing: s.clashing, hotTile: s.hotTile, builds: s.builds, ...(s.planned === undefined ? {} : { planned: s.planned }) });

const RAD = Math.PI / 180;

/** The extra gap, in degrees, at each end of a subagent's stretch. */
const WIDER = 1.5;

/** A slab: its south and east sides, then its top, so a near plot covers the plot behind it. */
function drawSlab(b: PlaneBuilder, tile: Tile, height: number, s: TilesIn, p: Palette): void {
  const { rect } = tile;
  const [x0, x1] = [rect.x, rect.x + rect.w];
  const [y0, y1] = [rect.y * FLAT, (rect.y + rect.h) * FLAT];
  const tint = (5 + rand(tile.path, 1) * 3) / 100;
  const clash = s.clashing.has(tile.path);
  const lit = s.lit.has(tile.path);
  const top = clash ? onGround(p, p.ask, 0.3) : onGround(p, p.fg, lit ? 0.17 : tint);
  const south = clash ? onGround(p, p.ask, 0.17) : onGround(p, p.fg, lit ? 0.08 : tint * 0.5);
  const east = clash ? onGround(p, p.ask, 0.1) : onGround(p, p.fg, lit ? 0.05 : tint * 0.3);
  const z = height;
  b.fan([{ x: x0, y: y1, z: 0 }, { x: x1, y: y1, z: 0 }, { x: x1, y: y1, z }, { x: x0, y: y1, z }], south);
  b.fan([{ x: x1, y: y0, z: 0 }, { x: x1, y: y1, z: 0 }, { x: x1, y: y1, z }, { x: x1, y: y0, z }], east);
  b.fan([{ x: x0, y: y0, z }, { x: x1, y: y0, z }, { x: x1, y: y1, z }, { x: x0, y: y1, z }], top);
  const edge = clash ? fade(p.ask, 0.75) : fade(p.fg, lit ? 0.34 : 0.18);
  b.stroke([{ x: x0, y: y0, z }, { x: x1, y: y0, z }, { x: x1, y: y1, z }, { x: x0, y: y1, z }], clash ? 0.9 : 0.55, edge, { closed: true });
  // Under the pointer, the top is ringed just inside its edge.
  if (s.hotTile === tile.path) b.stroke([{ x: x0 + 0.5, y: y0 + 0.5, z: z + 0.1 }, { x: x1 - 0.5, y: y0 + 0.5, z: z + 0.1 }, { x: x1 - 0.5, y: y1 - 0.5, z: z + 0.1 }, { x: x0 + 0.5, y: y1 - 0.5, z: z + 0.1 }], 0.7, fade(p.fg, 0.38), { closed: true });
  // The lip toward the viewer catches the light.
  b.stroke([{ x: x0, y: y1, z }, { x: x1, y: y1, z }], 0.6, fade(p.fg, clash ? 0.5 : 0.28));
  const inset = Math.min(rect.w, rect.h * FLAT);
  if (inset > 60) b.stroke(PlaneBuilder.rounded(x0 + 9, y0 + 9, rect.w - 18, rect.h * FLAT - 18, 12, z + 0.05), 0.5, fade(clash ? p.ask : p.fg, clash ? 0.3 : 0.08), { closed: true });
  drawPlan(b, tile, height, s.planned?.get(tile.path)?.piece, p);
  if (clash) {
    // Hatched, so it reads as a warning and not only as a colour.
    const [w, d] = [rect.w, y1 - y0];
    for (let k = 9; k < w + d; k += 9) {
      const [from, to] = [Math.max(0, (k - w) / d), Math.min(1, k / d)];
      if (from < to) b.stroke([{ x: x0 + k - d * from, y: y0 + d * from, z: z + 0.05 }, { x: x0 + k - d * to, y: y0 + d * to, z: z + 0.05 }], 0.6, fade(p.ask, 0.4));
    }
  }
}

/**
 * A plot the Session's plan names, framed just inside its edge in the plan bar's own language: a whole line and a light tint where the
 * work is done, an outline where it is done with no edits seen, amber where it is under way, a faint dashed line where it has not begun.
 */
function drawPlan(b: PlaneBuilder, tile: Tile, z: number, piece: PlotPiece | undefined, p: Palette): void {
  if (piece === undefined) return;
  const { rect } = tile;
  const [x0, x1] = [rect.x + 2, rect.x + rect.w - 2];
  const [y0, y1] = [rect.y * FLAT + 2, (rect.y + rect.h) * FLAT - 2];
  if (x1 <= x0 || y1 <= y0) return;
  const frame = [{ x: x0, y: y0, z: z + 0.06 }, { x: x1, y: y0, z: z + 0.06 }, { x: x1, y: y1, z: z + 0.06 }, { x: x0, y: y1, z: z + 0.06 }];
  const tint = piece === "solid" ? fade(p.fg, 0.1) : piece === "scaffold" ? fade(p.hot, 0.16) : piece === "failed" ? fade(p.bad, 0.16) : undefined;
  if (tint !== undefined) b.fan(frame, tint);
  if (piece === "solid") b.stroke(frame, 0.7, fade(p.fg, 0.7), { closed: true });
  else if (piece === "hollow") b.stroke(frame, 0.55, fade(p.fg, 0.8), { closed: true });
  else if (piece === "scaffold") b.stroke(frame, 0.7, p.hot, { closed: true });
  else if (piece === "failed") b.stroke(frame, 0.7, p.bad, { closed: true });
  else b.stroke(frame, 0.45, fade(p.subtle, 0.7), { closed: true, dash: [2, 3] });
}

/** The files of a plot, a pill for each, quiet on its top and rising with it. A file being worked on is a bar over its pill (`marks.ts`). */
function drawPills(b: PlaneBuilder, tile: Tile, height: number, p: Palette): void {
  const grid = tile.pills;
  if (grid === undefined) return;
  const tone = fade(p.fg, 0.2);
  const z = height + 0.2;
  for (let i = 0; i < grid.count; i += 1) {
    const r = pillRect(grid, i);
    b.quad(r.x, r.y * FLAT, r.w, r.h * FLAT, z, tone);
  }
}

/** The ring of a construction, laid on the ground round the point it stands at. */
function drawRing(b: PlaneBuilder, one: Placed, z: number, alpha: number, s: TilesIn, p: Palette): void {
  const { build } = one;
  const at = onPlane(one.anchor);
  const big = s.big;
  const [cx, cy] = [at.x, at.y];
  const shown = (c: Rgba, open = false): Rgba => fade(c, alpha * (open ? 1 - build.dim : 1));
  if (build.phase === "scope") drawSurvey(b, cx, cy, z, big, alpha, p);
  const n = Math.max(build.tasks.length, 1);
  const r1 = 34 * big;
  const r0 = r1 - 7 * big;
  const [a0, a1] = [r1 + 4 * big, r1 + 8.5 * big];
  const sector = (inner: number, outer: number, from: number, to: number): P3[] => [...PlaneBuilder.arc(cx, cy, outer, from * RAD, to * RAD, z, 12), ...PlaneBuilder.arc(cx, cy, inner, to * RAD, from * RAD, z, 12)];
  const solid = mix(p.base, p.fg, 0.24);

  const pieces = build.tasks;
  if (pieces.length === 0) {
    const band = bandOf(build);
    if (band !== undefined) {
      const open = band.state !== "solid";
      const edge = shown(fade(p.subtle, band.state === "outline" ? 0.4 : 0.8), open);
      if (band.state === "solid") b.disc(cx, cy, r1, shown(solid), z, { inner: r0 });
      b.stroke(PlaneBuilder.arc(cx, cy, r1, 0, Math.PI * 2, z, 48), 0.5, edge, { closed: true });
      b.stroke(PlaneBuilder.arc(cx, cy, r0, 0, Math.PI * 2, z, 48), 0.5, edge, { closed: true });
      if (band.state === "scaffold") b.stroke(PlaneBuilder.arc(cx, cy, (r0 + r1) / 2, 0, Math.PI * 2, z, 48), (r1 - r0) / 2, shown(fade(p.subtle, 0.5), true), { closed: true, dash: [1.4, 2.6] });
    }
  }
  const sameAsBefore = (k: number) => pieces[k]?.by === pieces[(k + n - 1) % n]?.by && pieces.length > 1;
  const edgeAt = (k: number) => (!sameAsBefore(k % n) && pieces.length > 1 ? WIDER : 0);
  pieces.forEach((piece, k) => {
    const from = -90 + (k * 360) / n + 2.5 + edgeAt(k);
    const to = -90 + ((k + 1) * 360) / n - 2.5 - edgeAt(k + 1);
    const path = sector(r0, r1, from, to);
    const open = piece.piece === "ghost" || piece.piece === "scaffold";
    const mid = ((from + to) / 2) * RAD;
    if (piece.piece === "solid") {
      b.disc(cx, cy, r1, shown(solid), z, { inner: r0, from: from * RAD, to: to * RAD });
      b.stroke(path, 0.5, shown(fade(p.subtle, 0.8)), { closed: true });
    } else if (piece.piece === "hollow") b.stroke(path, 0.7, shown(fade(p.fg, 0.8)), { closed: true });
    else b.stroke(path, 0.5, shown(fade(p.subtle, piece.piece === "scaffold" ? 0.9 : 0.6), open), { closed: true, dash: [2, 3] });
    if (piece.by !== undefined) b.disc(cx + ((r0 + r1) / 2) * Math.cos(mid), cy + ((r0 + r1) / 2) * Math.sin(mid), 1.7 * big, shown(fade(p.fg, 0.85)), z, { steps: 8 });
    if (piece.piece === "scaffold") b.stroke([{ x: cx + (r1 + 2) * Math.cos(mid), y: cy + (r1 + 2) * Math.sin(mid), z }, { x: cx + (r1 + 9) * Math.cos(mid), y: cy + (r1 + 9) * Math.sin(mid), z }], 0.5, shown(fade(p.hot, 0.75), true));
  });
  // A subagent's stretch is set off by a thin arc inside it.
  pieces.forEach((piece, k) => {
    if (piece.by === undefined || (k > 0 && pieces[k - 1]!.by === piece.by)) return;
    let length = 1;
    while (pieces[k + length]?.by === piece.by) length += 1;
    b.stroke(PlaneBuilder.arc(cx, cy, r0 - 2 * big, (-90 + (k * 360) / n + 2.5 + WIDER) * RAD, (-90 + ((k + length) * 360) / n - 2.5 - WIDER) * RAD, z, 16), 0.5, shown(fade(p.muted, 0.7)));
  });

  const road = roadOf(build);
  if (road !== undefined) {
    const m = Math.max(arcsOf(build).length, 1);
    road.forEach((arc, k) => {
      const from = -90 + (k * 360) / m + 3;
      const to = -90 + ((k + 1) * 360) / m - 3;
      const path = sector(a0, a1, from, to);
      if (arc.at === undefined) b.stroke(path, 0.5, shown(fade(p.subtle, 0.55)), { closed: true, dash: [2, 3] });
      else {
        b.disc(cx, cy, a1, shown(mix(p.base, p.accent, 0.6)), z, { inner: a0, from: from * RAD, to: to * RAD });
        b.stroke(path, 0.5, shown(p.accent), { closed: true });
      }
    });
  }
}

/** The survey a Session lays before it builds anything: a dashed plot with a faint grid and a stake at each corner. */
function drawSurvey(b: PlaneBuilder, cx: number, cy: number, z: number, big: number, alpha: number, p: Palette): void {
  const [w, d] = [96 * big, 62 * big];
  const [x0, y0] = [cx - w / 2, cy - d / 2];
  const grid = fade(p.fg, 0.09 * alpha);
  for (let x = 10; x < w; x += 10) b.stroke([{ x: x0 + x, y: y0, z }, { x: x0 + x, y: y0 + d, z }], 0.4, grid);
  for (let y = 10; y < d; y += 10) b.stroke([{ x: x0, y: y0 + y, z }, { x: x0 + w, y: y0 + y, z }], 0.4, grid);
  b.stroke([{ x: x0, y: y0, z }, { x: x0 + w, y: y0, z }, { x: x0 + w, y: y0 + d, z }, { x: x0, y: y0 + d, z }], 0.5, fade(p.fg, 0.38 * alpha), { closed: true, dash: [3, 3] });
  for (const [sx, sy] of [[-1, -1], [1, -1], [-1, 1], [1, 1]] as const) b.flat(cx + (sx * w) / 2 - 2, cy + (sy * d) / 2 - 2, 4, 4, z, fade(p.accent, alpha));
}

/**
 * The plots, their files and the rings, in painter's order. `heights` is where each plot stands now, which is not where it is going
 * while it eases; `ringAlpha` is how much of a construction is left, once it has landed.
 */
export function drawTiles(b: PlaneBuilder, s: TilesIn, p: Palette, heights: ReadonlyMap<string, number>, ringAlpha: (key: string) => number): void {
  const tiles = s.layout.territories.flatMap((one) => one.tiles).sort((a, c) => a.rect.y + a.rect.h - (c.rect.y + c.rect.h) || a.rect.x + a.rect.w - (c.rect.x + c.rect.w));
  for (const tile of tiles) {
    const height = heights.get(tile.path) ?? 0;
    drawSlab(b, tile, height, s, p);
    drawPills(b, tile, height, p);
  }
  for (const one of s.builds) {
    const alpha = ringAlpha(one.build.key);
    if (alpha <= 0) continue;
    drawRing(b, one, (one.tile === undefined ? 0 : (heights.get(one.tile) ?? 0)) + 1.5, alpha, s, p);
  }
}
