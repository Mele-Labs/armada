import { expect, test } from "vitest";
import type { DiffFile } from "@armada/components";

import { markCollisions } from "./clash-hunks";

const file = (...hunks: string[]): DiffFile => ({ path: "a.rs", lines: hunks.flatMap((text) => [{ kind: "hunk" as const, text }, { kind: "context" as const, text: " x" }]) });
const marked = (one: DiffFile, other: DiffFile) => markCollisions(one, other, "Collides").lines.filter((line) => line.kind === "hunk").map((line) => line.collides);

test("a hunk collides where the lines it changes meet the other side's", () => {
  expect(marked(file("@@ -10,5 +10,6 @@", "@@ -40,2 +41,3 @@"), file("@@ -12,3 +12,3 @@"))).toEqual(["Collides", undefined]);
});

test("ranges that only touch do not collide", () => {
  expect(marked(file("@@ -10,5 +10,6 @@"), file("@@ -15,2 +15,2 @@"))).toEqual([undefined]);
});

test("a hunk with no count is one line, and an insertion inside the other's range collides", () => {
  expect(marked(file("@@ -7 +7 @@"), file("@@ -7,0 +8,2 @@"))).toEqual(["Collides"]);
  expect(marked(file("@@ -20,0 +21,4 @@"), file("@@ -18,6 +18,6 @@"))).toEqual(["Collides"]);
});
