// The files a Job's drones are writing, lifted off the floor: a slab over each file's bar with the file's minimap on it, a row for each
// stretch of the file, lit green where lines were added and red where they were removed, and the bar's own place outlined, dashed, on
// the floor under it. `k` is how far it has risen, 0 on the floor to 1 up; it eases over a short rise and settles back the same way.
// A slab's size and rise are in pixels: `unit` is the plane's length of one at the camera's zoom, so it reads at any zoom.

import { BAR_LIFT, foldRows } from "../lens";
import { PlaneBuilder } from "./plane";
import { fade, mix, type Palette } from "./palette";
import type { LensSpec } from "./scene";

/** How thick a slab is. */
const THICK = 4;
/** The rows another writer's smaller slab draws. */
const SMALL_ROWS = 16;

const lerp = (a: number, b: number, t: number): number => a + (b - a) * t;

/** The same length for a row each time it is drawn: its file and its place, hashed. */
function lengthOf(path: string, k: number): number {
  let h = 2166136261 ^ Math.imul(k + 1, 40503);
  for (let i = 0; i < path.length; i += 1) h = Math.imul(h ^ path.charCodeAt(i), 16777619);
  h ^= h >>> 15;
  return (h >>> 0) / 4294967296;
}

export function drawLens(b: PlaneBuilder, lifted: readonly { spec: LensSpec; k: number }[], p: Palette, floorOf: (tile: string | undefined) => number, unit: number): void {
  for (const { spec, k } of lifted) {
    if (k <= 0) continue;
    const floor = floorOf(spec.tile) + BAR_LIFT;
    const edge = spec.card ? p.accent : mix(p.muted, p.fg, 0.2);
    // Its place on the floor, dashed, wider than a bar too small to see.
    const { mark } = spec;
    const [mw, mh] = [Math.max(mark.w + 3, 9), Math.max(mark.h + 3, 7)];
    const [mx, my] = [mark.x + mark.w / 2 - mw / 2, mark.y + mark.h / 2 - mh / 2];
    const z0 = floor + 0.2;
    b.stroke([{ x: mx, y: my, z: z0 }, { x: mx + mw, y: my, z: z0 }, { x: mx + mw, y: my + mh, z: z0 }, { x: mx, y: my + mh, z: z0 }], 0.6, fade(edge, 0.9 * k), { closed: true, dash: [3, 2.5] });
    // The slab grows out of the bar as it rises.
    const [w, d] = [lerp(Math.max(mark.w, 4), Math.max(mark.w, spec.w * unit), k), lerp(Math.max(mark.h, 4), Math.max(mark.h, spec.d * unit), k)];
    const [x0, y0, x1, y1] = [spec.at.x - w / 2, spec.at.y - d / 2, spec.at.x + w / 2, spec.at.y + d / 2];
    const top = floor + spec.up * unit * k;
    const under = top - THICK * unit * k;
    b.stroke([{ ...spec.at, z: floor }, { ...spec.at, z: under }], 0.45, fade(edge, 0.5 * k), { dash: [2, 3] });
    const side = fade(mix(p.base, p.fg, 0.05), k);
    const shade = fade(mix(p.base, p.fg, 0.02), k);
    b.fan([{ x: x0, y: y0, z: under }, { x: x1, y: y0, z: under }, { x: x1, y: y0, z: top }, { x: x0, y: y0, z: top }], shade);
    b.fan([{ x: x0, y: y0, z: under }, { x: x0, y: y1, z: under }, { x: x0, y: y1, z: top }, { x: x0, y: y0, z: top }], shade);
    b.fan([{ x: x1, y: y0, z: under }, { x: x1, y: y1, z: under }, { x: x1, y: y1, z: top }, { x: x1, y: y0, z: top }], side);
    b.fan([{ x: x0, y: y1, z: under }, { x: x1, y: y1, z: under }, { x: x1, y: y1, z: top }, { x: x0, y: y1, z: top }], side);
    b.flat(x0, y0, w, d, top, fade(mix(p.base, p.fg, 0.1), k));
    b.stroke([{ x: x0, y: y0, z: top }, { x: x1, y: y0, z: top }, { x: x1, y: y1, z: top }, { x: x0, y: y1, z: top }], spec.card ? 0.7 : 0.5, fade(edge, k), { closed: true });
    // The minimap: a row for each stretch of the file, lit where the change is.
    const rows = spec.card ? spec.rows : foldRows(spec.rows, SMALL_ROWS);
    const [mx0, mw0] = [x0 + w * 0.1, w * 0.8];
    const [ry0, rd] = [y0 + d * 0.07, d * 0.86];
    const z = top + 0.15;
    rows.forEach((row, i) => {
      const y = ry0 + ((i + 0.5) * rd) / rows.length;
      const h = lengthOf(spec.path, i);
      if (row === 0) {
        b.stroke([{ x: mx0, y, z }, { x: mx0 + mw0 * (0.25 + 0.55 * h), y, z }], 0.3, fade(p.muted, 0.45 * k));
        return;
      }
      const long = mx0 + mw0 * (0.55 + 0.4 * h);
      const half = spec.card ? 0.95 : 0.8;
      if (row === 3) {
        const mid = lerp(mx0, long, 0.55);
        b.stroke([{ x: mx0, y, z }, { x: mid, y, z }], half, fade(p.good, k));
        b.stroke([{ x: mid, y, z }, { x: long, y, z }], half, fade(p.bad, k));
      } else b.stroke([{ x: mx0, y, z }, { x: long, y, z }], half, fade(row === 1 ? p.good : p.bad, k));
    });
  }
}
