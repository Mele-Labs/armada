// When a bucket of a Job's time is marked far over its usual: twice the usual and ten minutes past it, both. Each line is held
// from both sides, since a mark on a Job that ran a little long reads as the page crying wolf.

import { farOver } from "@armada/components/src/compositions/UsagePage/usage-time";
import { expect, test } from "vitest";

const MINUTE = 60;

test("twice the usual and ten minutes past it is far over", () => {
  expect(farOver(40 * MINUTE, 20 * MINUTE)).toBe(true);
  // Exactly at both lines.
  expect(farOver(20 * MINUTE, 10 * MINUTE)).toBe(true);
});

test("just under twice the usual is not, though it is more than ten minutes past", () => {
  expect(farOver(39 * MINUTE + 59, 20 * MINUTE)).toBe(false);
});

test("just under ten minutes past the usual is not, though it is more than twice", () => {
  expect(farOver(13 * MINUTE, 4 * MINUTE)).toBe(false);
  expect(farOver(14 * MINUTE - 1, 4 * MINUTE)).toBe(false);
  expect(farOver(14 * MINUTE, 4 * MINUTE)).toBe(true);
});

test("a workflow with no usual marks nothing", () => {
  expect(farOver(500 * MINUTE, undefined)).toBe(false);
});
