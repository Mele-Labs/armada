import { expect, test } from "vitest";

import type { PullRequest } from "./IssuesBlock";
import { stackedRows } from "./pull-stacks";

const pr = (number: number, branch: string, base?: string, state: PullRequest["state"] = "open"): PullRequest => ({
  kind: "pull_request",
  number,
  title: `Change ${number}`,
  branch,
  address: `https://git.example/pull/${number}`,
  checks: { state: "passed" },
  state,
  auto: false,
  ...(base === undefined ? {} : { base }),
});

const shape = (rows: ReturnType<typeof stackedRows>) => rows.map((row) => [row.pr.number, row.lane === "none" ? "none" : `${row.lane.first ? "first" : ""}${row.lane.last ? "last" : ""}` || "mid", row.chip]);

test("pull requests on no other keep today's order, newest first, with no lane", () => {
  const rows = stackedRows([pr(1, "a", "main"), pr(2, "b", "main"), pr(3, "c")]);
  expect(rows.map((row) => row.pr.number)).toEqual([3, 2, 1]);
  expect(rows.every((row) => row.lane === "none" && row.rootBase === undefined && row.chip === undefined)).toBe(true);
});

test("a chain of five reads parent first, one lane from the first row to the last, and no chip where the parent is the row above", () => {
  const rows = stackedRows([pr(5, "e", "d"), pr(3, "c", "b"), pr(1, "a", "main"), pr(4, "d", "c"), pr(2, "b", "a")]);
  expect(shape(rows)).toEqual([
    [1, "first", undefined],
    [2, "mid", undefined],
    [3, "mid", undefined],
    [4, "mid", undefined],
    [5, "last", undefined],
  ]);
  expect(rows[0]?.rootBase).toBe("main");
  expect(rows[1]?.rootBase).toBeUndefined();
  expect(rows[2]?.parent).toBe(2);
});

test("a second child of one parent follows the first child's own stack and names its parent in a chip", () => {
  const rows = stackedRows([pr(1, "root", "main"), pr(2, "one", "root"), pr(3, "two", "root"), pr(4, "one-deep", "one")]);
  expect(shape(rows)).toEqual([
    [1, "first", undefined],
    [2, "mid", undefined],
    [4, "mid", undefined],
    [3, "last", 1],
  ]);
});

test("a stack sits where its root sat among the pull requests on none", () => {
  const rows = stackedRows([pr(1, "alone"), pr(2, "root", "main"), pr(3, "child", "root"), pr(4, "other")]);
  expect(rows.map((row) => row.pr.number)).toEqual([4, 2, 3, 1]);
  expect(rows.map((row) => row.lane === "none")).toEqual([true, false, false, true]);
});

test("a merged parent drops the lane and the chip of its children", () => {
  const rows = stackedRows([pr(1, "gone", "main", "merged"), pr(2, "child", "gone"), pr(3, "other-child", "gone")]);
  expect(rows.map((row) => row.pr.number)).toEqual([3, 2, 1]);
  expect(rows.every((row) => row.lane === "none" && row.chip === undefined && row.rootBase === undefined && row.parent === undefined)).toBe(true);
});

test("two pull requests each on the other still draw, as unstacked", () => {
  const rows = stackedRows([pr(1, "a", "b"), pr(2, "b", "a")]);
  expect(rows.map((row) => row.pr.number).sort()).toEqual([1, 2]);
  expect(rows.every((row) => row.lane === "none")).toBe(true);
});
