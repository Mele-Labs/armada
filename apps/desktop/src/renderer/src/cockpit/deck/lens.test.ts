import { expect, test } from "vitest";

import { ROWS, cardOf, countsOf, foldRows, lensOf, newestOf, writersOf } from "./lens";

const head = (path: string) => `diff --git a/${path} b/${path}\n--- a/${path}\n+++ b/${path}\n`;
const diff = (grown: number) => `${head("a.ts")}@@ -8,2 +8,2 @@\n keep\n-old\n+new\n@@ -40,1 +40,${1 + grown} @@\n end\n${Array.from({ length: grown }, (_, k) => `+line ${k}\n`).join("")}`;

test("the counts, the hunks and the rows are read once from the diff, lit where the changed lines sit", () => {
  const read = lensOf({ ok: true, stamp: "s1", diff: diff(3) });
  expect(countsOf(read)).toBe("+4 −1");
  expect(read.hunks.map((one) => one.old)).toEqual([8, 40]);
  expect(read.hunks[1]!.lines.map((one) => one.text)).toEqual(["end", "line 0", "line 1", "line 2"]);
  // Lines 9 and 41 to 43 of a file the change reaches to line 43.
  const lit = read.rows.flatMap((row, k) => (row === 0 ? [] : [k]));
  expect(read.rows[Math.floor((8 / 43) * ROWS)]).toBe(3);
  expect(lit.at(-1)).toBe(ROWS - 1);
  expect(read.cut).toBeUndefined();
});

test("a diff Fleet cut says how many lines came", () => {
  expect(lensOf({ ok: true, stamp: "s", diff: diff(1), cut: true }).cut).toBe("First 7 lines");
});

test("a smaller slab folds its rows in pairs, lit where either was", () => {
  expect(foldRows([0, 1, 0, 0, 2, 1, 0, 0], 4)).toEqual([1, 0, 3, 0]);
});

test("the newest hunk is the one that grew, and only the lines new since the last answer are unread", () => {
  const first = lensOf({ ok: true, stamp: "1", diff: diff(1) });
  expect(newestOf(undefined, first)).toMatchObject({ hunk: { old: 40 }, read: 2 });
  const grown = lensOf({ ok: true, stamp: "2", diff: diff(3) });
  expect(newestOf(first, grown)).toMatchObject({ hunk: { old: 40 }, read: 2 });
  expect(newestOf(grown, grown)).toBeUndefined();
});

test("the card follows the drone that started writing last, and a pin holds it while that drone writes", () => {
  const was = writersOf(new Map(), [{ key: "main", path: "a.ts" }], 1);
  const now = writersOf(was, [{ key: "main", path: "b.ts" }, { key: "sub", path: "c.ts" }], 5);
  // Moving to another file is no new start.
  expect(now.get("main")!.since).toBe(1);
  const writers = [...now.values()];
  expect(cardOf(writers, undefined)?.key).toBe("sub");
  expect(cardOf(writers, "main")?.path).toBe("b.ts");
});

test("a pinned drone that stops writing hands the card back to whoever writes", () => {
  const writers = [...writersOf(new Map(), [{ key: "main", path: "a.ts" }, { key: "sub", path: "c.ts" }], 5).values()];
  expect(cardOf(writers, "gone")?.key).toBe("sub");
  expect(cardOf([], "gone")).toBeUndefined();
});
