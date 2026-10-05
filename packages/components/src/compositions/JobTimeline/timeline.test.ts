import { describe, expect, it } from "vitest";

import { clampWindow, following, LEAST_WIDTH_MS, packRows, percentOf, reaches, spanOf, ticksOf, zoomed } from "./timeline";

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
