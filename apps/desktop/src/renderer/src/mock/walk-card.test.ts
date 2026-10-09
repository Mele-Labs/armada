import { describe, expect, test } from "vitest";

import { keepInside, placeCard } from "./walk-card";

const view = { width: 1440, height: 900 };
const size = { width: 340, height: 200 };
const box = (left: number, top: number, width: number, height: number) => ({ left, top, right: left + width, bottom: top + height });

describe("where the walk's card rests", () => {
  test("at the bottom while the ring is clear of it", () => {
    expect(placeCard(box(100, 100, 200, 40), size, view, 24, 60)).toBe("bottom");
    expect(placeCard(null, size, view, 24, 60)).toBe("bottom");
  });

  test("at the top, under the top bar, when the ring is under the bottom corner", () => {
    // The merge-line band's right end: main's light and the dots nearest it.
    expect(placeCard(box(1100, 850, 300, 30), size, view, 24, 60)).toBe("top");
  });

  test("at the bottom still when the ring is under both corners, since the top one is no better", () => {
    expect(placeCard(box(1000, 0, 440, 900), size, view, 24, 60)).toBe("bottom");
  });

  test("a ring beside the card, not under it, leaves it where it was", () => {
    expect(placeCard(box(900, 850, 100, 30), size, view, 24, 60)).toBe("bottom");
  });
});

describe("a dragged card", () => {
  test("stays inside the window", () => {
    expect(keepInside({ x: -50, y: 2000 }, size, view)).toEqual({ x: 0, y: 700 });
    expect(keepInside({ x: 300, y: 300 }, size, view)).toEqual({ x: 300, y: 300 });
  });
});
