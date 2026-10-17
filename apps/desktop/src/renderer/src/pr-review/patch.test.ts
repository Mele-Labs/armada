// Where a drawn line is in the file, and the line of a patch a forge's place names: the one conversion the review
// surface makes between the diff's indexes and the forge's (path, line, side).

import { expect, test } from "vitest";

import { indexOfPlace, linesOfPatch, patchOfLines, placeOfLine } from "./patch";

const PATCH = ["@@ -10,4 +10,5 @@ fn read()", " keep", "-old one", "-old two", "+new one", "+new two", "+new three", " tail"].join("\n");

test("a patch is drawn as the forge printed it, and prints back the same", () => {
  const lines = linesOfPatch("a.rs", PATCH);
  expect(lines.map((one) => one.kind)).toEqual(["hunk", "context", "removed", "removed", "added", "added", "added", "context"]);
  expect(patchOfLines(lines)).toBe(PATCH);
});

test("a removed line is on the left by its old number, anything the head has is on the right by its new one", () => {
  const lines = linesOfPatch("a.rs", PATCH);
  expect(placeOfLine(lines, 1)).toEqual({ line: 10, side: "right" });
  expect(placeOfLine(lines, 2)).toEqual({ line: 11, side: "left" });
  expect(placeOfLine(lines, 3)).toEqual({ line: 12, side: "left" });
  expect(placeOfLine(lines, 4)).toEqual({ line: 11, side: "right" });
  expect(placeOfLine(lines, 6)).toEqual({ line: 13, side: "right" });
  expect(placeOfLine(lines, 7)).toEqual({ line: 14, side: "right" });
});

test("the hunk header and a line past the end have no place", () => {
  const lines = linesOfPatch("a.rs", PATCH);
  expect(placeOfLine(lines, 0)).toBeUndefined();
  expect(placeOfLine(lines, 99)).toBeUndefined();
});

test("a place is found again by its index, and a context line answers to either side", () => {
  const lines = linesOfPatch("a.rs", PATCH);
  expect(indexOfPlace(lines, { line: 12, side: "left" })).toBe(3);
  expect(indexOfPlace(lines, { line: 13, side: "right" })).toBe(6);
  expect(indexOfPlace(lines, { line: 10, side: "right" })).toBe(1);
  expect(indexOfPlace(lines, { line: 10, side: "left" })).toBe(1);
  expect(indexOfPlace(lines, { line: 99, side: "right" })).toBe(-1);
});

test("a second hunk counts from its own header", () => {
  const lines = linesOfPatch("a.rs", [PATCH, "@@ -40,2 +41,2 @@", " same", "-gone", "+came"].join("\n"));
  expect(placeOfLine(lines, 9)).toEqual({ line: 41, side: "right" });
  expect(placeOfLine(lines, 10)).toEqual({ line: 41, side: "left" });
  expect(placeOfLine(lines, 11)).toEqual({ line: 42, side: "right" });
});
