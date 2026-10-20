// Where a sketch's boxes and arrows go once the boxes are measured: `settle.ts` and `route.ts` in
// `packages/components/src/compositions/SketchScene/`. The scene is the one an agent drew of the auto-merge
// notice (20 Oct 2026), placed for 240 by 90 boxes and drawn with boxes far taller than that.

import { describe, expect, it } from "vitest";

import { route } from "@armada/components/src/compositions/SketchScene/route";
import { settle, type Box } from "@armada/components/src/compositions/SketchScene/settle";

type Rect = { x: number; y: number; w: number; h: number };

const GAP = 32;

/** As the agent placed them, at the sizes the canvas measured. */
const NOTICE: Box[] = [
  { id: "queues", x: 0, y: 0, w: 240, h: 118 },
  { id: "auto", x: 320, y: 0, w: 260, h: 140 },
  { id: "left", x: 640, y: 0, w: 280, h: 118 },
  { id: "again", x: 0, y: 170, w: 240, h: 118 },
  { id: "reads", x: 320, y: 170, w: 300, h: 230 },
  { id: "needs", x: 640, y: 170, w: 260, h: 96 },
  { id: "fix", x: 320, y: 340, w: 320, h: 200 },
];

const EDGES = [
  ["queues", "auto"],
  ["auto", "left"],
  ["auto", "again"],
  ["again", "reads"],
  ["reads", "needs"],
  ["reads", "fix"],
] as const;

/** Closer than `gap` on both axes, so one box sits on or against another. */
const touching = (a: Rect, b: Rect, gap: number) => a.x < b.x + b.w + gap && b.x < a.x + a.w + gap && a.y < b.y + b.h + gap && b.y < a.y + a.h + gap;

/** Whether the straight run from `a` to `b` passes through the inside of `r`. */
function cuts(a: { x: number; y: number }, b: { x: number; y: number }, r: Rect): boolean {
  const left = Math.min(a.x, b.x);
  const right = Math.max(a.x, b.x);
  const top = Math.min(a.y, b.y);
  const bottom = Math.max(a.y, b.y);
  return left < r.x + r.w - 1 && right > r.x + 1 && top < r.y + r.h - 1 && bottom > r.y + 1;
}

describe("settling a sketch", () => {
  it("leaves no two boxes closer than the gap", () => {
    const placed = settle(NOTICE, GAP);
    const rects = NOTICE.map((one) => placed.get(one.id)!);
    for (const [i, one] of rects.entries()) for (const other of rects.slice(i + 1)) expect(touching(one, other, GAP - 1)).toBe(false);
  });

  it("keeps the agent's columns and rows in their order", () => {
    const placed = settle(NOTICE, GAP);
    const at = (id: string) => placed.get(id)!;
    expect(at("queues").x).toBeLessThan(at("auto").x);
    expect(at("auto").x).toBeLessThan(at("left").x);
    expect(at("queues").y).toBe(at("auto").y);
    expect(at("auto").y).toBeLessThan(at("reads").y);
    expect(at("reads").y).toBeLessThan(at("fix").y);
    expect(at("auto").x).toBe(at("reads").x);
  });

  it("leaves a sketch that already has room where the agent put it", () => {
    const roomy: Box[] = [
      { id: "a", x: 0, y: 0, w: 240, h: 90 },
      { id: "b", x: 400, y: 0, w: 240, h: 90 },
    ];
    const placed = settle(roomy, GAP);
    expect(placed.get("a")).toEqual({ x: 0, y: 0, w: 240, h: 90 });
    expect(placed.get("b")).toEqual({ x: 400, y: 0, w: 240, h: 90 });
  });

  it("grows a group round the boxes it held", () => {
    const boxes: Box[] = [
      { id: "lane", x: -20, y: -40, w: 600, h: 160, ground: true },
      { id: "a", x: 0, y: 0, w: 380, h: 100 },
      { id: "b", x: 300, y: 0, w: 380, h: 100 },
    ];
    const placed = settle(boxes, GAP);
    const lane = placed.get("lane")!;
    for (const id of ["a", "b"]) {
      const one = placed.get(id)!;
      expect(one.x).toBeGreaterThanOrEqual(lane.x);
      expect(one.x + one.w).toBeLessThanOrEqual(lane.x + lane.w);
      expect(one.y).toBeGreaterThanOrEqual(lane.y);
      expect(one.y + one.h).toBeLessThanOrEqual(lane.y + lane.h);
    }
  });
});

describe("routing an arrow", () => {
  it("runs clear of every box it does not start or end at", () => {
    const placed = settle(NOTICE, GAP);
    for (const [from, to] of EDGES) {
      const others = NOTICE.filter((one) => one.id !== from && one.id !== to).map((one) => placed.get(one.id)!);
      const points = route(placed.get(from)!, placed.get(to)!, others, GAP);
      for (let at = 1; at < points.length; at += 1) for (const box of others) expect(cuts(points[at - 1]!, points[at]!, box), `${from} to ${to} cuts a box`).toBe(false);
    }
  });

  it("runs through neither of its own boxes between its ends", () => {
    const placed = settle(NOTICE, GAP);
    for (const [from, to] of EDGES) {
      const others = NOTICE.filter((one) => one.id !== from && one.id !== to).map((one) => placed.get(one.id)!);
      const points = route(placed.get(from)!, placed.get(to)!, others, GAP);
      // The first and last runs leave and reach a side; everything between stays outside both boxes.
      for (let at = 2; at < points.length - 1; at += 1) {
        expect(cuts(points[at - 1]!, points[at]!, placed.get(from)!)).toBe(false);
        expect(cuts(points[at - 1]!, points[at]!, placed.get(to)!)).toBe(false);
      }
    }
  });

  it("starts on the edge of its first box and ends on the edge of its last", () => {
    const placed = settle(NOTICE, GAP);
    const on = (p: { x: number; y: number }, r: Rect) =>
      ((p.x === r.x || p.x === r.x + r.w) && p.y >= r.y && p.y <= r.y + r.h) || ((p.y === r.y || p.y === r.y + r.h) && p.x >= r.x && p.x <= r.x + r.w);
    for (const [from, to] of EDGES) {
      const points = route(placed.get(from)!, placed.get(to)!, [], GAP);
      expect(on(points[0]!, placed.get(from)!)).toBe(true);
      expect(on(points[points.length - 1]!, placed.get(to)!)).toBe(true);
    }
  });
});
