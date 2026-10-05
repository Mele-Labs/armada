import { describe, expect, it } from "vitest";

import { clampWindow, following, laneRows, LEAST_WIDTH_MS, packRows, percentOf, reaches, spanOf, ticksOf, zoomed } from "./timeline";

const MIN = 60_000;
const span = { start: 0, width: 600 * MIN };

describe("packRows", () => {
  it("shares a row between bars that do not overlap and splits ones that do", () => {
    const rows = packRows([
      { id: "a", from: 0, to: 10 },
      { id: "b", from: 5, to: 15 },
      { id: "c", from: 10, to: 20 },
    ]);
    expect([rows.get("a"), rows.get("b"), rows.get("c")]).toEqual([0, 1, 0]);
  });

  it("puts a child in its parent's row when that is free", () => {
    const rows = packRows([
      { id: "p", from: 0, to: 10 },
      { id: "x", from: 2, to: 30 },
      { id: "c", from: 12, to: 20, parent: "p" },
      { id: "y", from: 11, to: 40 },
    ]);
    expect(rows.get("c")).toBe(rows.get("p"));
  });

  it("puts a child in the free row nearest its parent's when its own is taken", () => {
    const rows = packRows([
      { id: "a", from: 0, to: 100 },
      { id: "b", from: 1, to: 100 },
      { id: "p", from: 2, to: 100 },
      { id: "q", from: 3, to: 100 },
      { id: "x", from: 4, to: 9 },
      { id: "c", from: 5, to: 30, parent: "p" },
    ]);
    expect([rows.get("p"), rows.get("c")]).toEqual([2, 4]);
  });

  it("holds 150 chained and fanned-out jobs in far fewer rows", () => {
    const bars = Array.from({ length: 150 }, (_, at) => ({ id: `${at}`, from: at * 10, to: at * 10 + 25 }));
    expect(new Set(packRows(bars).values()).size).toBeLessThanOrEqual(3);
  });
});

describe("window", () => {
  it("spans from the first start to now for a live bar", () => {
    expect(spanOf([{ from: 100, to: 200 }], 900_000)).toEqual({ start: 100, width: 900_000 - 100 });
  });

  it("never narrows below the least width or leaves the span", () => {
    expect(clampWindow({ start: -5, width: 1 }, span)).toEqual({ start: 0, width: LEAST_WIDTH_MS });
    expect(clampWindow({ start: 590 * MIN, width: 60 * MIN }, span).start).toBe(540 * MIN);
  });

  it("keeps the focus where it was on screen when zooming", () => {
    const win = { start: 0, width: 600 * MIN };
    const next = zoomed(win, span, 2, 300 * MIN);
    expect(next.width).toBe(300 * MIN);
    expect(percentOf(next, 300 * MIN)).toBeCloseTo(50);
  });

  it("pans only when the playhead leaves it", () => {
    const win = { start: 100 * MIN, width: 100 * MIN };
    expect(following(win, span, 150 * MIN)).toBe(win);
    expect(following(win, span, 300 * MIN).start).toBeGreaterThan(100 * MIN);
    expect(reaches(following(win, span, 300 * MIN), 300 * MIN, 300 * MIN)).toBe(true);
  });
});

describe("ticks", () => {
  it("lands on round times and gives about the number asked for", () => {
    const ticks = ticksOf({ start: 7 * MIN, width: 120 * MIN }, 8);
    expect(ticks.every((t) => t % (15 * MIN) === 0)).toBe(true);
    expect(ticks.length).toBeGreaterThan(3);
    expect(ticks.length).toBeLessThanOrEqual(9);
  });
});

describe("laneRows", () => {
  const bar = (id: string, from: number, to: number) => ({ id, from, to });

  it("keeps a label off the span of the Job before it, and a Job beyond that on the same row", () => {
    const { rows, count } = laneRows([bar("a", 0, 100), bar("b", 30, 60), bar("c", 200, 220)], new Map(), 20);
    expect([rows.get("a"), rows.get("b"), rows.get("c")]).toEqual([0, 1, 0]);
    expect(count).toBe(2);
  });

  it("holds a short Job's row for the width of its label", () => {
    const { rows } = laneRows([bar("a", 0, 5), bar("b", 10, 15)], new Map(), 20);
    expect(rows.get("b")).toBe(1);
  });

  it("is no rows for no Jobs", () => {
    expect(laneRows([], new Map(), 20).count).toBe(0);
  });
});
