// The browser project runs with reduced motion emulated, and `motion()` turns it back on for one
// test. `vitest.config.ts` says why; this is what shows it took.

import { beforeEach, describe, expect, test } from "vitest";

import { motion } from "./testing";

const REDUCED = "(prefers-reduced-motion: reduce)";

test("a test runs with reduced motion unless it asks for motion", () => {
  expect(window.matchMedia(REDUCED).matches).toBe(true);
});

test("motion() turns it on for the test that asks", async () => {
  await motion();
  expect(window.matchMedia(REDUCED).matches).toBe(false);
});

test("and the next test is back to reduced", () => {
  expect(window.matchMedia(REDUCED).matches).toBe(true);
});

describe("asked for in beforeEach", () => {
  beforeEach(motion);

  test("motion is on for every test under it", () => {
    expect(window.matchMedia(REDUCED).matches).toBe(false);
  });
});

test("and reduced again once they are done", () => {
  expect(window.matchMedia(REDUCED).matches).toBe(true);
});
