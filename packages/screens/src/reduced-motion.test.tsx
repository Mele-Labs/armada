// The browser project runs with reduced motion emulated, and `motion()` turns it back on for one
// test. `vitest.config.ts` says why; this is what shows it took.

import { expect, test } from "vitest";

import { motion } from "./mounted";

const REDUCED = "(prefers-reduced-motion: reduce)";

const sheet = (): string => getComputedStyle(document.documentElement).getPropertyValue("--duration-sheet").trim();

test("a test runs with reduced motion unless it asks for motion", () => {
  expect(window.matchMedia(REDUCED).matches).toBe(true);
  // The tokens' own answer to it: a sheet travels in no time at all.
  expect(sheet()).toBe("0ms");
});

test("motion() turns it on for the test that asks", async () => {
  await motion();
  expect(window.matchMedia(REDUCED).matches).toBe(false);
  expect(sheet()).not.toBe("0ms");
});

test("and the next test is back to reduced", () => {
  expect(window.matchMedia(REDUCED).matches).toBe(true);
  expect(sheet()).toBe("0ms");
});
