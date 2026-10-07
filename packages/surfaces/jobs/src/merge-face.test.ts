import { expect, test } from "vitest";

import { checksFactOf, mergeFaceOf } from "./merge-face";

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
  expect(checksFactOf({ kind: "some_failed", checks: 2, failed: ["a", "b"] })).toBe("Checks failed: a, b");
});

test("the checks as the card says them", () => {
  expect(checksFactOf({ kind: "all_passed", checks: 2 })).toBe("Checks passed");
  expect(checksFactOf({ kind: "still_waiting", checks: 2 })).toBe("Checks running");
  expect(checksFactOf({ kind: "nothing_ran", checks: 0 })).toBeUndefined();
});
