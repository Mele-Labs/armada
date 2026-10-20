// Where a sketch's boxes and arrows go once the boxes are measured: `settle.ts` and `route.ts` in
// `packages/components/src/compositions/SketchScene/`. The scene is the one an agent drew of the auto-merge
// notice (20 Oct 2026), placed for 240 by 90 boxes and drawn with boxes far taller than that.

import { describe, expect, it } from "vitest";

import { labelSize, placeLabels } from "@armada/components/src/compositions/SketchScene/edge-labels";
import { roundedPath, route, routeAll } from "@armada/components/src/compositions/SketchScene/route";
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

/** The data-flow hub from the first walk (20 Oct 2026): arrows converging on Fleet, words on every arrow. */
const HUB: Box[] = [
  { id: "you", x: 118, y: -80, w: 32, h: 22 },
  { id: "bridge", x: 0, y: 0, w: 340, h: 124 },
  { id: "sessions", x: 520, y: 0, w: 320, h: 124 },
  { id: "check", x: -420, y: 300, w: 320, h: 124 },
  { id: "fleet", x: 0, y: 300, w: 320, h: 170 },
  { id: "sqlite", x: 520, y: 300, w: 240, h: 124 },
  { id: "drone-a", x: -120, y: 590, w: 320, h: 110 },
  { id: "drone-b", x: 240, y: 590, w: 320, h: 110 },
];
const HUB_EDGES = [
  { id: "e0", from: "you", to: "bridge" },
  { id: "e1", from: "bridge", to: "fleet", label: "WS /jobs/:id/observe and run observe, HTTP for commands" },
  { id: "e2", from: "sessions", to: "fleet", label: "MCP: get_events_since(upto), get_job, start_run, show_sketch" },
  { id: "e3", from: "fleet", to: "check", label: "code: advances, retry, fails" },
  { id: "e4", from: "fleet", to: "sqlite", label: "every transition" },
  { id: "e5", from: "drone-a", to: "fleet", label: "evidence via Armada's tool" },
  { id: "e6", from: "drone-b", to: "fleet", label: "evidence via Armada's tool" },
];

const overlaps = (a: Rect, b: Rect) => a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;

describe("several arrows", () => {
  it("into one side of a box arrive apart, never at one point", () => {
    const rects = settle(HUB, GAP);
    const ways = routeAll(rects, HUB_EDGES, GAP);
    const ends = HUB_EDGES.map((one) => ways.get(one.id)!.at(-1)!);
    const starts = HUB_EDGES.map((one) => ways.get(one.id)![0]!);
    const all = [...ends, ...starts];
    for (const [i, one] of all.entries()) for (const other of all.slice(i + 1)) expect(Math.hypot(one.x - other.x, one.y - other.y)).toBeGreaterThan(8);
  });

  it("keep their words off every box and off every arrow's head", () => {
    const m = { charW: 14 * 0.56, lineH: 20, pad: 4 };
    const joins = HUB_EDGES.flatMap((one) => (one.label === undefined ? [] : [{ from: one.from, to: one.to, ...labelSize(one.label, m) }]));
    const rects = settle(HUB, GAP, joins);
    const ways = routeAll(rects, HUB_EDGES, GAP);
    const drawn = HUB_EDGES.map((one) => ({ id: one.id, label: one.label, path: roundedPath(ways.get(one.id)!, 8) }));
    const heads = [...ways.values()].map((points) => ({ x: points.at(-1)!.x - 8, y: points.at(-1)!.y - 8, w: 16, h: 16 }));
    const places = placeLabels(drawn, [...rects.values()], m, heads);
    expect(places.size).toBe(6);
    for (const [id, p] of places) {
      const words = { x: p.x - p.w / 2, y: p.y - p.h / 2, w: p.w, h: p.h };
      for (const [box, r] of rects) expect(overlaps(words, r), `${id}'s words on ${box}`).toBe(false);
      for (const head of heads) expect(overlaps(words, head), `${id}'s words on an arrow's head`).toBe(false);
    }
  });
});
