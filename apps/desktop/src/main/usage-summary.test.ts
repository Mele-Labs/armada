// The usage summary is one request, and what crosses from the renderer is only an offset in minutes.

import { expect, test } from "vitest";

import { usageSummaryPath } from "./usage-summary";
import { offsetOf } from "./usage-summary-channels";

test("the path carries the caller's offset and nothing else", () => {
  expect(usageSummaryPath(120)).toBe("/usage/summary?tz_offset_minutes=120");
  expect(usageSummaryPath(-300)).toBe("/usage/summary?tz_offset_minutes=-300");
});

test("only a whole offset within a day either way is passed on", () => {
  expect(offsetOf(-300)).toBe(-300);
  expect(offsetOf(0)).toBe(0);
  for (const bad of [1441, -1441, 1.5, "120", null, undefined, NaN]) expect(offsetOf(bad)).toBeNull();
});
