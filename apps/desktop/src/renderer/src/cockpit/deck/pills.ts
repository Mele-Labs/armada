// The pills that fill a plot: one for each file its directory holds, in a grid that depends on the file count and the plot's size, and
// on nothing else, so a file is always in the same place. The grid is cut once per layout; the canvas draws it, and the pointer finds a
// pill by arithmetic on the plot, since a repository has thousands. Pure.

import type { Rect } from "./layout";

/**
 * A plot's pills. Everything is in the layout's own units, where a depth is `FLAT` times shorter than it is drawn. Pill `i` stands in
 * cell `i % cols` across and `i / cols` down; `names` are the first of them, relative to the plot, and the rest are unnamed.
 */
export type PillGrid = {
  count: number;
  names: readonly string[];
  cols: number;
  /** The top left of the first cell, and the size of a cell. */
  x: number;
  y: number;
  cw: number;
  ch: number;
  /** The size of a pill, centred in its cell. */
  pw: number;
  ph: number;
};

/** How wide a pill may be drawn against its depth, and the most a pill grows to in a plot with few files, in the plane's own units. */
const WIDEST = 3.2;
const SQUAREST = 1.6;
const BIGGEST = { w: 60, d: 24 };
/** The least space between pills. */
const GAP = 0.7;

const clamp = (n: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, n));

/**
 * The grid for `count` files on a plot, or none where the plot is too small to hold one. The columns are the count that gives the largest
 * pill, so five files in a roomy plot are five good-sized pills and nine hundred are a fine texture. `flat` is how much deeper the plane
 * draws than the layout counts.
 */
export function gridOf(rect: Rect, count: number, names: readonly string[], flat: number): PillGrid | undefined {
  if (count <= 0) return undefined;
  const deep = rect.h * flat;
  const pad = clamp(Math.min(rect.w, deep) * 0.06, 2, 5);
  // A plot's name lies along its far edge when there is room for it.
  const head = deep > 56 ? 12 : 3;
  const width = rect.w - pad * 2;
  const depth = deep - head - pad;
  if (width < 3 || depth < 1.5) return undefined;
  let best: { cols: number; rows: number; w: number; d: number; score: readonly [number, number] } | undefined;
  for (let cols = 1; cols <= Math.min(count, Math.floor(width / (GAP * 2))); cols += 1) {
    const rows = Math.ceil(count / cols);
    const [cw, cd] = [width / cols, depth / rows];
    const gap = Math.max(GAP, 0.16 * Math.min(cw, cd));
    const [w0, d0] = [cw - gap, cd - gap];
    if (w0 <= 0 || d0 <= 0) continue;
    // Held between a wide pill and a squarer one, then no larger than a pill is worth being.
    let [w, d] = [w0, d0];
    if (w0 / d0 > WIDEST) w = d0 * WIDEST;
    else if (w0 / d0 < SQUAREST) d = w0 / SQUAREST;
    const grow = Math.min(1, BIGGEST.w / w, BIGGEST.d / d);
    const score = [w * grow * (d * grow), w * d] as const;
    if (best === undefined || score[0] > best.score[0] + 1e-9 || (Math.abs(score[0] - best.score[0]) <= 1e-9 && score[1] > best.score[1])) best = { cols, rows, w: w * grow, d: d * grow, score };
  }
  if (best === undefined) return undefined;
  return {
    count,
    names,
    cols: best.cols,
    x: rect.x + pad,
    y: rect.y + head / flat,
    cw: width / best.cols,
    ch: depth / best.rows / flat,
    pw: best.w,
    ph: best.d / flat,
  };
}

/** Where pill `i` stands. */
export function pillRect(grid: PillGrid, i: number): Rect {
  const [col, row] = [i % grid.cols, Math.floor(i / grid.cols)];
  return { x: grid.x + col * grid.cw + (grid.cw - grid.pw) / 2, y: grid.y + row * grid.ch + (grid.ch - grid.ph) / 2, w: grid.pw, h: grid.ph };
}

/** The pill whose cell holds this point of the layout, or none for a point outside the grid or in the empty cells at its end. */
export function pillAt(grid: PillGrid, x: number, y: number): number | undefined {
  const [col, row] = [Math.floor((x - grid.x) / grid.cw), Math.floor((y - grid.y) / grid.ch)];
  if (col < 0 || col >= grid.cols || row < 0) return undefined;
  const i = row * grid.cols + col;
  return i < grid.count ? i : undefined;
}
