// The widths of the deck's side columns: clamped to the window as it is now, and the centre keeps its floor.

import { expect, test } from "vitest";

import { settle } from "./columns";

const UNIT = 16;

test("a column never dragged rests at its own width", () => {
  const { left, right } = settle({}, 100 * UNIT, UNIT);
  expect([left, right]).toEqual([6 * UNIT, 5.75 * UNIT]);
});

test("a dragged width is kept while the window has room for it and gives way when it does not", () => {
  expect(settle({ left: 20 * UNIT }, 100 * UNIT, UNIT).left).toBe(20 * UNIT);
  const narrow = settle({ left: 20 * UNIT }, 30 * UNIT, UNIT);
  expect(narrow.left).toBeLessThan(20 * UNIT);
  expect(30 * UNIT - narrow.left - narrow.right).toBeGreaterThanOrEqual(10 * UNIT - 1);
});

test("neither column goes under its floor", () => {
  const { left, right } = settle({ left: 1, right: 1 }, 100 * UNIT, UNIT);
  expect(left).toBe(4.5 * UNIT);
  expect(right).toBe(4.75 * UNIT);
});
