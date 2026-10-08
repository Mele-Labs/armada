import { expect, test } from "vitest";

import { forgeChecksOf, mergeFaceOf } from "./merge-face";

test("nothing ran, unreadable and unread are plain Merge", () => {
  expect(mergeFaceOf(undefined, undefined)).toEqual({ how: "merge" });
  expect(mergeFaceOf({ kind: "nothing_ran", checks: 0 }, undefined)).toEqual({ how: "merge" });
  expect(mergeFaceOf({ kind: "unreadable", checks: 0 }, undefined)).toEqual({ how: "merge" });
});

test("a failed run blocks and names what failed", () => {
  expect(mergeFaceOf({ kind: "some_failed", checks: 2, failed: ["a", "b"] }, undefined)).toEqual({
    how: "merge",
    blocked: "a, b failed",
  });
  expect(forgeChecksOf({ kind: "some_failed", checks: 2, failed: ["a", "b"] })).toEqual({ checks: "failed", failing: ["a", "b"] });
});

test("the checks as the card's marks take them", () => {
  expect(forgeChecksOf({ kind: "all_passed", checks: 2 })).toEqual({ checks: "passed" });
  expect(forgeChecksOf({ kind: "still_waiting", checks: 2 })).toEqual({ checks: "running" });
  expect(forgeChecksOf({ kind: "nothing_ran", checks: 0 })).toEqual({});
});
