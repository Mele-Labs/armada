import { expect, test } from "vitest";

import { covers, sideOf } from "./LensWires";

const right = { l: 600, t: 12, r: 900, b: 200 };
const left = { l: 12, t: 12, r: 312, b: 200 };

test("the card moves to the top left only when the right covers its drone or slab and the left does not, and does not toss back", () => {
  expect(covers(right, [{ x: 700, y: 100 }])).toBe(true);
  expect(sideOf("right", right, left, [{ x: 500, y: 300 }])).toBe("right");
  expect(sideOf("right", right, left, [{ x: 700, y: 100 }])).toBe("left");
  // Under both, it stays where it is.
  expect(sideOf("right", right, left, [{ x: 700, y: 100 }, { x: 100, y: 100 }])).toBe("right");
  expect(sideOf("left", right, left, [{ x: 500, y: 300 }])).toBe("left");
  expect(sideOf("left", right, left, [{ x: 100, y: 100 }])).toBe("right");
});
