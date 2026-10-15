// Where the callout's card stands from the thing it is about: up and to the right by default, over on the side that has room at
// the pane's edges, never clipped and never over what it names, with a dotted leader running to its near corner.

import { expect, test } from "vitest";

import { EDGE, GAP, placeCallout } from "./placement";

const PANE = { width: 900, height: 600 };
const CARD = { w: 240, h: 130 };
const thing = (x: number, y: number) => ({ x, y, w: 60, h: 40 });
const covers = (a: { left: number; top: number }, at: { x: number; y: number; w: number; h: number }) =>
  a.left < at.x + at.w / 2 && a.left + CARD.w > at.x - at.w / 2 && a.top < at.y + at.h / 2 && a.top + CARD.h > at.y - at.h / 2;

test("with room it stands up and to the right, a gap off the thing", () => {
  const at = thing(300, 400);
  const put = placeCallout(at, CARD, PANE);
  expect(put.right).toBe(true);
  expect(put.up).toBe(true);
  expect(put.left).toBeCloseTo(at.x + at.w / 2 + GAP, 0);
  expect(put.top + CARD.h).toBeLessThanOrEqual(at.y - at.h / 2 - GAP + 0.5);
});

test("at the right edge it stands to the left, and at the top it stands below", () => {
  const right = placeCallout(thing(860, 400), CARD, PANE);
  expect(right.right).toBe(false);
  expect(right.left + CARD.w).toBeLessThan(860);

  const top = placeCallout(thing(300, 40), CARD, PANE);
  expect(top.up).toBe(false);
  expect(top.top).toBeGreaterThan(40);
});

test("the card is inside the pane whatever corner the thing is in, and never over the thing", () => {
  for (const x of [20, 450, 880]) {
    for (const y of [20, 300, 580]) {
      const at = thing(x, y);
      const put = placeCallout(at, CARD, PANE);
      expect(put.left).toBeGreaterThanOrEqual(EDGE);
      expect(put.top).toBeGreaterThanOrEqual(EDGE);
      expect(put.left + CARD.w).toBeLessThanOrEqual(PANE.width - EDGE);
      expect(put.top + CARD.h).toBeLessThanOrEqual(PANE.height - EDGE);
      expect(covers(put, at)).toBe(false);
    }
  }
});

test("the leader ends on the card: at its near corner, or on its near side where the card has been pushed alongside", () => {
  const corner = placeCallout(thing(300, 400), CARD, PANE);
  expect(corner.end.x).toBe(corner.left);
  expect(corner.end.y).toBe(corner.top + CARD.h);

  // Pushed down against the top edge, its side faces the thing: the leader meets the side, level with it.
  const side = placeCallout(thing(300, 60), CARD, PANE);
  expect(side.end.x).toBe(side.left);
  expect(side.end.y).toBeGreaterThanOrEqual(side.top);
  expect(side.end.y).toBeLessThanOrEqual(side.top + CARD.h);
  // It starts a little way off the thing, on the line to its end.
  expect(Math.hypot(side.start.x - 300, side.start.y - 60)).toBeGreaterThan(0);
});

test("a pane smaller than the card still gets a card at its edge and no negative place", () => {
  const put = placeCallout(thing(50, 50), CARD, { width: 200, height: 100 });
  expect(put.left).toBeGreaterThanOrEqual(EDGE);
  expect(put.top).toBeGreaterThanOrEqual(EDGE);
});
