// The way an arrow takes between two boxes: square runs through the room between boxes, never across one.
// A curve drawn straight from side to side went through whatever box sat between, and under it, so the
// arrow's end could not be followed. No layout library: a sketch has a few dozen arrows, and trying each
// pair of sides with each run that the gaps between boxes offer is enough.

import type { Rect } from "./edge-labels";

export type Point = { x: number; y: number };
type Side = "top" | "right" | "bottom" | "left";

const SIDES: readonly Side[] = ["right", "left", "bottom", "top"];
const OUT: Record<Side, Point> = { top: { x: 0, y: -1 }, right: { x: 1, y: 0 }, bottom: { x: 0, y: 1 }, left: { x: -1, y: 0 } };

const sideOf = (r: Rect, side: Side): Point =>
  side === "top" ? { x: r.x + r.w / 2, y: r.y } : side === "bottom" ? { x: r.x + r.w / 2, y: r.y + r.h } : side === "left" ? { x: r.x, y: r.y + r.h / 2 } : { x: r.x + r.w, y: r.y + r.h / 2 };

/** Whether the run from `a` to `b` passes through the inside of `r`; running along its edge does not. */
function cuts(a: Point, b: Point, r: Rect): boolean {
  return Math.min(a.x, b.x) < r.x + r.w - 1 && Math.max(a.x, b.x) > r.x + 1 && Math.min(a.y, b.y) < r.y + r.h - 1 && Math.max(a.y, b.y) > r.y + 1;
}

/** Corners where the way goes straight on are dropped, and so are runs of no length. */
function tidy(points: readonly Point[]): Point[] {
  const kept: Point[] = [];
  for (const one of points) {
    const last = kept[kept.length - 1];
    if (last !== undefined && last.x === one.x && last.y === one.y) continue;
    const before = kept[kept.length - 2];
    if (before !== undefined && last !== undefined && ((before.x === last.x && last.x === one.x) || (before.y === last.y && last.y === one.y))) kept.pop();
    kept.push(one);
  }
  return kept;
}

/**
 * The corners of the way from `from` to `to`, its ends on their sides. `others` are the boxes it must not
 * cross; its own two are kept out of everything but the first and last runs.
 */
export function route(from: Rect, to: Rect, others: readonly Rect[], gap: number): Point[] {
  const stub = gap / 2;
  // Where a run between the boxes may go: halfway, and just outside any box near the two.
  const near = [from, to, ...others];
  const lanes = (key: "x" | "y", size: "w" | "h") => {
    const lo = Math.min(from[key], to[key]) - gap * 2;
    const hi = Math.max(from[key] + from[size], to[key] + to[size]) + gap * 2;
    const all = near.flatMap((one) => [one[key] - stub, one[key] + one[size] + stub]);
    return [...new Set(all.filter((one) => one >= lo && one <= hi))];
  };
  const xs = lanes("x", "w");
  const ys = lanes("y", "h");

  let best: Point[] | undefined;
  let bestCost = Number.POSITIVE_INFINITY;
  for (const out of SIDES) {
    for (const into of SIDES) {
      const start = sideOf(from, out);
      const end = sideOf(to, into);
      const a = { x: start.x + OUT[out].x * stub, y: start.y + OUT[out].y * stub };
      const b = { x: end.x + OUT[into].x * stub, y: end.y + OUT[into].y * stub };
      const middles: Point[][] = [
        [{ x: b.x, y: a.y }],
        [{ x: a.x, y: b.y }],
        [{ x: (a.x + b.x) / 2, y: a.y }, { x: (a.x + b.x) / 2, y: b.y }],
        [{ x: a.x, y: (a.y + b.y) / 2 }, { x: b.x, y: (a.y + b.y) / 2 }],
        ...xs.map((x) => [{ x, y: a.y }, { x, y: b.y }]),
        ...ys.map((y) => [{ x: a.x, y }, { x: b.x, y }]),
      ];
      for (const middle of middles) {
        const points = tidy([start, a, ...middle, b, end]);
        let cost = 0;
        for (let at = 1; at < points.length && cost < bestCost; at += 1) {
          const p = points[at - 1]!;
          const q = points[at]!;
          cost += Math.abs(q.x - p.x) + Math.abs(q.y - p.y) + 24;
          for (const box of others) if (cuts(p, q, box)) cost += 100_000;
          const inner = at > 1 && at < points.length - 1;
          if (inner && (cuts(p, q, from) || cuts(p, q, to))) cost += 100_000;
          // The first run leaves its box and the last reaches its own: back through either is a crossing too.
          if (at === 1 && cuts(p, q, to)) cost += 100_000;
          if (at === points.length - 1 && cuts(p, q, from)) cost += 100_000;
        }
        if (cost < bestCost) {
          bestCost = cost;
          best = points;
        }
      }
    }
  }
  return best ?? [sideOf(from, "right"), sideOf(to, "left")];
}

/** The way as an SVG path, its corners rounded to `radius` or as much as the runs either side allow. */
export function roundedPath(points: readonly Point[], radius: number): string {
  const num = (n: number) => String(Math.round(n * 10) / 10);
  const [first] = points;
  if (first === undefined) return "";
  let d = `M${num(first.x)} ${num(first.y)}`;
  for (let at = 1; at < points.length - 1; at += 1) {
    const p = points[at - 1]!;
    const c = points[at]!;
    const n = points[at + 1]!;
    const into = Math.hypot(c.x - p.x, c.y - p.y);
    const away = Math.hypot(n.x - c.x, n.y - c.y);
    const r = Math.min(radius, into / 2, away / 2);
    const s = { x: c.x - ((c.x - p.x) / into) * r, y: c.y - ((c.y - p.y) / into) * r };
    const e = { x: c.x + ((n.x - c.x) / away) * r, y: c.y + ((n.y - c.y) / away) * r };
    d += ` L${num(s.x)} ${num(s.y)} Q${num(c.x)} ${num(c.y)} ${num(e.x)} ${num(e.y)}`;
  }
  const last = points[points.length - 1]!;
  return `${d} L${num(last.x)} ${num(last.y)}`;
}
