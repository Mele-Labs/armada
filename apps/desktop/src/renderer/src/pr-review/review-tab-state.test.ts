// Where a comment on lines picked in the Review tab's gutter goes: one line, or a range on the side of the last.

import { expect, test } from "vitest";

import { linesOfPatch } from "./patch";
import { placeOfPick } from "./review-tab-state";

const LINES = linesOfPatch(
  "a.rs",
  [
    "@@ -10,4 +10,5 @@ fn read()",
    " keep",
    "-old one",
    "-old two",
    "+new one",
    "+new two",
    "+new three",
    " tail",
  ].join("\n"),
);

test("one picked line is that line, with no start", () => {
  expect(
    placeOfPick(LINES, {
      from: 4,
      to: 4,
    }),
  ).toEqual({
    line: 11,
    side: "right",
  });
});

test("picked lines start at the first one on the side of the last", () => {
  expect(
    placeOfPick(LINES, {
      from: 2,
      to: 6,
    }),
  ).toEqual({
    line: 13,
    side: "right",
    startLine: 11,
  });
  expect(
    placeOfPick(LINES, {
      from: 1,
      to: 3,
    }),
  ).toEqual({
    line: 12,
    side: "left",
    startLine: 10,
  });
});

test("a context line starts a range on the right", () => {
  expect(
    placeOfPick(LINES, {
      from: 1,
      to: 5,
    }),
  ).toEqual({
    line: 12,
    side: "right",
    startLine: 10,
  });
});
