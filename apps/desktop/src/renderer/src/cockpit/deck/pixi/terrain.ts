// The ground of the map: the grid the field stands on, each territory's front and the border round each top directory, the bases and
// their pads, the objective's zone and. Everything here lies flat, so
// it is drawn first and never moves with the plots' heights; it is rebuilt only when the layout, the bays, or the flags move.

import { onPlane } from "../bf-parts";
import { FLAT, type P3, type Point, type Territory } from "../layout";
import type { PadIn } from "../tip";
import { PlaneBuilder } from "./plane";
import { fade, mix, type Palette, type Rgba } from "./palette";
import type { PixiSceneIn } from "./scene";

const flatAt = (p: Point, z = 0): P3 => ({ ...onPlane(p), z });

/** The grid under the field: lines every 72 units, faint, and gone toward the edge of a wide ellipse, as the CSS masked it. */
function drawFloor(b: PlaneBuilder, s: PixiSceneIn, p: Palette): void {
  const { plane } = s;
  const box = { x: -plane.width * 0.6, y: -plane.height * 0.9, w: plane.width * 2.2, h: plane.height * 2.1 };
  const [cx, cy] = [box.x + box.w / 2, box.y + box.h * 0.48];
  const strength = (x: number, y: number) => {
    const t = Math.hypot((x - cx) / (box.w / 2), (y - cy) / (box.h / 2));
    return t <= 0.22 ? 1 : t >= 0.74 ? 0 : 1 - (t - 0.22) / 0.52;
  };
  const line = fade(p.fg, 0.1);
  const shade = (q: P3): Rgba => fade(line, strength(q.x, q.y));
  const STEP = 72;
  const along = (from: P3, to: P3) => {
    // Cut into pieces so the fade follows the line, and keep the runs that are not yet gone.
    const parts = Math.max(1, Math.ceil(Math.hypot(to.x - from.x, to.y - from.y) / 48));
    let run: P3[] = [];
    for (let i = 0; i <= parts; i += 1) {
      const q = { x: from.x + ((to.x - from.x) * i) / parts, y: from.y + ((to.y - from.y) * i) / parts, z: -1 };
      if (strength(q.x, q.y) > 0.01) run.push(q);
      else if (run.length > 1) {
        b.stroke(run, 0.5, line, { shade });
        run = [];
      } else run = [];
    }
    if (run.length > 1) b.stroke(run, 0.5, line, { shade });
  };
  for (let x = box.x; x <= box.x + box.w; x += STEP) along({ x, y: box.y, z: -1 }, { x, y: box.y + box.h, z: -1 });
  for (let y = box.y; y <= box.y + box.h; y += STEP) along({ x: box.x, y, z: -1 }, { x: box.x + box.w, y, z: -1 });
}

/** A territory: its floor, lighter toward the viewer, its front in dots, and a border round each top directory. */
function drawTerritory(b: PlaneBuilder, one: Territory, p: Palette): void {
  const { rect } = one;
  const front = PlaneBuilder.rounded(rect.x, rect.y * FLAT, rect.w, rect.h * FLAT, 30, 0);
  const far = rect.y * FLAT;
  const deep = rect.h * FLAT;
  b.shaded(front, front.map((q) => fade(p.fg, 0.012 + 0.04 * ((q.y - far) / deep))));
  b.stroke(front, 0.9, fade(p.subtle, 0.7), { closed: true, dash: [2.4, 6.1] });
  for (const group of one.groups) {
    const g = group.rect;
    b.stroke(PlaneBuilder.rounded(g.x, g.y * FLAT, g.w, g.h * FLAT, 8, 0), 0.8, fade(p.fg, 0.22), { closed: true });
  }
}

/** The compound: two plates, the outer dashed and the inner whole. */
function drawBase(b: PlaneBuilder, one: Territory, p: Palette): void {
  const { base } = one;
  const centre = onPlane({ x: base.x, y: base.y + 4 });
  const width = base.w + 24;
  const outer = PlaneBuilder.rounded(centre.x - width / 2, centre.y - 27 * FLAT, width, 54 * FLAT, 36, 0);
  b.fan(outer, fade(p.fg, 0.05));
  b.stroke(outer, 0.9, fade(p.subtle, 0.6), { closed: true, dash: [3, 5] });
  const inner = PlaneBuilder.rounded(centre.x - width / 2 + 16, centre.y - 19 * FLAT, width - 32, 38 * FLAT, 26, 0);
  b.fan(inner, fade(p.fg, 0.04));
  b.stroke(inner, 0.8, fade(p.subtle, 0.3), { closed: true });
}

/** A bay's landing pad: dim where free, lit where leased, amber where stranded, dark where closed. */
function drawPad(b: PlaneBuilder, pad: PadIn, at: P3, hot: boolean, p: Palette): void {
  const bay = pad.bay;
  const lit = bay === "held" || bay === "kept" || bay === "busy" || bay === "leased";
  const stranded = bay === "stranded";
  const closed = bay === "closed";
  const tone = stranded ? p.ask : p.accent;
  const hold = bay === "free" ? 0.55 : closed ? 0.75 : 1;
  const ring: Rgba = hot ? p.fg : lit || stranded ? tone : fade(p.subtle, 0.7);
  const plate: Rgba = lit ? fade(p.accent, 0.16) : stranded ? fade(p.ask, 0.18) : closed ? fade(mix(p.base, [0, 0, 0, 1], 0.85), 0.9) : fade(p.fg, 0.04);
  if (lit || stranded) b.halo(at.x, at.y, 13, 30, fade(tone, 0.3 * hold), fade(tone, 0), 0);
  b.disc(at.x, at.y, 13, fade(plate, hold), 0);
  const outline = PlaneBuilder.arc(at.x, at.y, 13, 0, Math.PI * 2, 0, 28);
  b.stroke(outline, lit || stranded ? 1 : 0.7, fade(ring, hot ? 1 : hold * (closed ? 0.6 : 1)), { closed: true, ...(bay === "ghost" ? { dash: [2, 3] as const } : {}) });
  const core = PlaneBuilder.arc(at.x, at.y, 6.5, 0, Math.PI * 2, 0, 20);
  if (lit || stranded) b.disc(at.x, at.y, 6.5, fade(tone, hold), 0);
  else b.stroke(core, 0.5, fade(p.subtle, (closed ? 0.3 : 0.4) * hold), { closed: true });
}

/** The objective's zone: a dotted circle round a lit rim, green while main is green and red while it is not. */
function drawGoal(b: PlaneBuilder, at: P3, red: boolean, p: Palette): void {
  const tone = red ? p.bad : p.good;
  b.stroke(PlaneBuilder.arc(at.x, at.y, 64, 0, Math.PI * 2, 0, 56), 0.7, fade(p.subtle, 0.8), { closed: true, dash: [1.2, 4.2] });
  b.halo(at.x, at.y, 28, 50, fade(tone, 0.45), fade(tone, 0), 0);
  b.halo(at.x, at.y, 14, 28, fade(tone, 0), fade(tone, 0.22), 0);
  b.disc(at.x, at.y, 28, fade(p.fg, 0.08), 0);
  b.stroke(PlaneBuilder.arc(at.x, at.y, 28, 0, Math.PI * 2, 0, 40), 0.9, tone, { closed: true });
}

export function drawTerrain(b: PlaneBuilder, s: PixiSceneIn, p: Palette): void {
  drawFloor(b, s, p);
  for (const one of s.layout.territories) {
    drawTerritory(b, one, p);
    drawBase(b, one, p);
    for (const pad of one.pads) {
      const info = s.pads.get(pad.key);
      if (info !== undefined) drawPad(b, info, flatAt(pad), s.hotPart === `pad:${pad.key}`, p);
    }
    const goal = s.goals.get(one.root);
    if (goal !== undefined && one.goal !== undefined) drawGoal(b, flatAt(one.goal), goal.red, p);
  }
}
