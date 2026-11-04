// What the Builds panel works out from a build: the change a figure made against the build before, and the clock a build began
// on. Both are absent or quiet where there is nothing real to read, so each boundary is held from both sides.

import {
  changeOf,
  commitOf,
  shown,
  startedAt,
  valueOf,
} from "@armada/components/src/compositions/UsagePage/usage-builds";
import type { Build } from "@armada/components/src/compositions/UsagePage/usage-builds";
import { expect, test } from "vitest";

import { usageBuildsNow } from "./mock/usage-builds";

/** Any build the mock holds, for the fields a change never reads. */
const base = usageBuildsNow(Date.parse("2026-11-03T15:00:00Z"))[0]!;
const build = (turns: number, checks = 100): Build => ({
  ...base,
  turns,
  checks,
});

test("a change shows at 5% and is still just under it", () => {
  const before = build(100, 100);
  expect(changeOf(build(100, 105), before, "checks")).toMatchObject({
    still: false,
    up: true,
    percent: 5,
  });
  expect(changeOf(build(100, 104), before, "checks")).toMatchObject({
    still: true,
  });
  expect(changeOf(build(100, 95), before, "checks")).toMatchObject({
    still: false,
    up: false,
    percent: 5,
  });
});

test("a build, or the one before it, with under 30 turns shows no change", () => {
  const before = build(100, 100);
  expect(changeOf(build(30, 150), before, "checks")).toMatchObject({
    still: false,
    percent: 50,
  });
  expect(changeOf(build(29, 150), before, "checks")).toBeUndefined();
  expect(changeOf(build(100, 150), build(30, 100), "checks")).toMatchObject({
    still: false,
    percent: 50,
  });
  expect(changeOf(build(100, 150), build(29, 100), "checks")).toBeUndefined();
});

test("the oldest build has nothing before it, and a figure the build before never had reads as still", () => {
  expect(changeOf(build(100, 150), undefined, "checks")).toBeUndefined();
  expect(changeOf(build(100, 150), build(100, 0), "checks")).toMatchObject({
    still: true,
  });
});

test("a build began reads as month, day and the clock, in the reader's own zone", () => {
  // Built from local fields, so the expected clock is the same on any machine's zone.
  expect(startedAt(new Date(2026, 10, 3, 14, 36).toISOString())).toBe("Nov 3 14:36");
  expect(startedAt(new Date(2026, 0, 9, 9, 5).toISOString())).toBe("Jan 9 09:05");
});

/** A build that produced none of a figure: a figure Fleet had no sample for is absent, never 0. */
const without = (turns: number, figure: "checks" | "burn"): Build => {
  const { [figure]: _gone, ...rest } = build(turns);
  return rest;
};

test("a build with no figure draws a dash, not a number that is not one", () => {
  expect(valueOf(without(100, "checks"), "checks")).toBeUndefined();
  expect(shown(without(100, "checks"), "checks")).toBe("–");
  expect(shown(without(100, "burn"), "burn")).toBe("–");
  expect(shown(build(100, 41), "checks")).toBe("41s");
});

test("a change needs the figure on both builds, so a missing one is never a NaN percent", () => {
  expect(changeOf(without(100, "checks"), build(100, 100), "checks")).toBeUndefined();
  expect(changeOf(build(100, 100), without(100, "checks"), "checks")).toBeUndefined();
  expect(changeOf(without(100, "burn"), build(100), "burn")).toBeUndefined();
});

test("a build's commit is the first seven characters of its id, however much more tells it from another", () => {
  expect(
    commitOf({
      ...base,
      id: "4be7c19@2026-11-03T09:00:00Z",
    }),
  ).toBe("4be7c19");
  expect(
    commitOf({
      ...base,
      id: "d03a6e1",
    }),
  ).toBe("d03a6e1");
});
