// The ground of the map: the grid the field stands on, each territory's front and the border round each top directory, the bases and
// their pads, the objective's zone and, on a Job's mission, the route's legs and the disc under each pin. Everything here lies flat, so
// it is drawn first and never moves with the plots' heights; it is rebuilt only when the layout, the bays, the flags or the route move.

import { onPlane } from "../bf-parts";
import { FLAT, type P3, type Point, type Territory } from "../layout";
import type { PinState } from "../mission";
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

const PIN_TONE: Record<PinState, (p: Palette) => Rgba> = { done: (p) => p.good, live: (p) => p.hot, next: (p) => p.dormant, failed: (p) => p.bad, ask: (p) => p.ask };

/** The disc a pin stands on, lying on the plot. */
function drawPinDisc(b: PlaneBuilder, at: P3, state: PinState, pinned: boolean, hot: boolean, p: Palette): void {
  const tone = PIN_TONE[state](p);
  const r = 26;
  if (state === "done") {
    b.halo(at.x, at.y, r, r + 16, fade(tone, 0.5), fade(tone, 0), 1);
    b.halo(at.x, at.y, 0.01, r, fade(tone, 0.46), fade(tone, 0.14), 1);
  } else if (state === "live") {
    b.halo(at.x, at.y, r, r + 20, fade(tone, 0.55), fade(tone, 0), 1);
    b.halo(at.x, at.y, 0.01, r, fade(tone, 0.36), fade(tone, 0.04), 1);
  } else if (state === "failed") b.disc(at.x, at.y, r, fade(tone, 0.22), 1);
  else if (state === "ask") {
    b.disc(at.x, at.y, r, fade(tone, 0.1), 1);
    // Rings going out, held at one radius: the beacon's motion belongs to the effects.
    b.stroke(PlaneBuilder.arc(at.x, at.y, r * 1.5, 0, Math.PI * 2, 1, 48), 1, fade(tone, 0.5), { closed: true });
  } else b.disc(at.x, at.y, r, fade(tone, 0.1), 1);
  b.stroke(PlaneBuilder.arc(at.x, at.y, r, 0, Math.PI * 2, 1, 40), 0.8, fade(pinned || hot ? p.fg : tone, 0.7 + (pinned || hot ? 0.3 : 0)), { closed: true, ...(state === "next" ? { dash: [4, 3] as const } : {}) });
  if (pinned) b.stroke(PlaneBuilder.arc(at.x, at.y, r + 3, 0, Math.PI * 2, 1, 40), 1, p.accent, { closed: true });
}

/** The line a leg draws across the ground: a shallow bow, so a run of legs reads as a path and not a ruler. */
function bow(from: Point, to: Point): P3[] {
  const [a, c] = [onPlane(from), onPlane(to)];
  const lift = Math.min(18, Math.hypot(c.x - a.x, c.y - a.y) * 0.08);
  const [mx, my] = [(a.x + c.x) / 2, (a.y + c.y) / 2 - lift];
  const steps = 24;
  return Array.from({ length: steps + 1 }, (_, i) => {
    const t = i / steps;
    const u = 1 - t;
    return { x: u * u * a.x + 2 * u * t * mx + t * t * c.x, y: u * u * a.y + 2 * u * t * my + t * t * c.y, z: 0.5 };
  });
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
  const route = s.route;
  if (route === undefined) return;
  for (const leg of route.legs) {
    const path = bow(leg.from, leg.to);
    if (leg.back) b.stroke(path, 0.8, fade(p.accent, 0.75), { dash: [5, 6] });
    else b.stroke(path, 1.1, leg.done ? fade(p.good, 0.85) : fade(p.subtle, 0.6), { dash: [2.4, 5.6] });
  }
  for (const pin of route.pins) drawPinDisc(b, flatAt(pin.at, 0), pin.state, s.pinned === pin.id, s.hotPart === `pin:${pin.id}`, p);
}
