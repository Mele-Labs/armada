// A step of a playing sketch is as long as `--duration-pulse`, and a built stylesheet writes 1600ms as 1.6s.

import { expect, test } from "vitest";

import { cssMs } from "@armada/components/src/compositions/SketchScene/scene";

test("a CSS time is read in its own unit", () => {
  expect(cssMs("1600ms")).toBe(1600);
  expect(cssMs("1.6s")).toBe(1600);
  expect(cssMs(" .6s")).toBe(600);
  expect(cssMs("")).toBeUndefined();
  expect(cssMs("calc(1s)")).toBeUndefined();
});
