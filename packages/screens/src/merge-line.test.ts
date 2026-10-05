// The merge line folded from what Fleet serves onto the panel's rows.
//
// `merge-lines.served.json` is the answer `crates/fleet/src/tests/merge_lines.rs` reads back off a
// real `armada-land/` directory and holds equal to this file, so this is the other half of one
// reading of the wire.

import type { MergeLines } from "@armada/protocol";
import { GitMerge } from "lucide-react";
import { describe, expect, test } from "vitest";

import served from "./fixtures/build/merge-lines.served.json";
import { mergeLineViews } from "./merge-line";

const SERVED = served as MergeLines;
const PULL = "https://git.example/armada-dev/armada/pull/";
const REPOSITORIES = [
  { root: "/repo", records_root: "/records/repo" },
  { root: "/other", records_root: "/records/other" },
];
const views = (lines: MergeLines | null, picked: string | null) => mergeLineViews(lines, picked, REPOSITORIES);

describe("the merge line Fleet serves", () => {
  test("folds onto the panel's rows: places, the batch, the runner's words, what landed and what was sent back", () => {
    expect(views(SERVED, null)).toEqual([{
      root: "/repo",
      line: [
        {
          branch: "docs/wire-lock-signed",
          place: 1,
          state: "preparing",
          doing: "reading verify-foundations against main",
          batch: "docs/wire-lock-signed",
        },
        {
          branch: "worktree-agent-a",
          place: 2,
          state: "gating",
          batch: "docs/wire-lock-signed",
          checks: [
            { name: "build", state: "passed" },
            { name: "screens_test", state: "running" },
            { name: "desktop_test", state: "waiting" },
          ],
        },
        {
          branch: "fleet/gate-policy-every-run",
          place: 3,
          state: "preparing",
          doing: "merging main (c527f60e09) into fleet/gate-policy-every-run",
          batch: "docs/wire-lock-signed",
        },
        { branch: "fleet/push-the-base", place: 4, state: "merging", doing: "pushing the merge onto main" },
        { branch: "fleet/helm-kills-processes", place: 5, state: "waiting" },
        {
          branch: "fleet/read-in-cluster-membership",
          place: 6,
          pr: { number: 1770, url: `${PULL}1770` },
          state: "waiting",
        },
      ],
      landed: [
        {
          branch: "bridge/land-board-reads-plainly",
          pr: {
            number: 1769,
            url: `${PULL}1769`,
            settled: { status: "completed-success", icon: GitMerge, label: "Merged" },
          },
          state: "landed",
          merge: "29064cc27a",
        },
        { branch: "fleet/an-older-landing", state: "landed" },
      ],
      sentBack: [
        { branch: "bridge/overview-strip-width", state: "conflict", conflicts: ["apps/desktop/src/renderer/src/App.tsx"] },
        {
          branch: "fleet/pulse-log-rows",
          pr: { number: 1768, url: `${PULL}1768` },
          state: "red",
          failed: ["desktop_test", "screens_test"],
          checks: [
            { name: "build", state: "passed" },
            { name: "desktop_test", state: "failed" },
            { name: "screens_test", state: "timed_out" },
          ],
        },
      ],
    }]);
  });

  test("a turn that has run no Check yet reads as preparing, and one in its Checks as gating", () => {
    const [one] = views(SERVED, "/repo");
    expect(one?.line.map((row) => [row.branch, row.state])).toEqual([
      ["docs/wire-lock-signed", "preparing"],
      ["worktree-agent-a", "gating"],
      ["fleet/gate-policy-every-run", "preparing"],
      ["fleet/push-the-base", "merging"],
      ["fleet/helm-kills-processes", "waiting"],
      ["fleet/read-in-cluster-membership", "waiting"],
    ]);
  });

  test("the line is ordered by what merges next, and the number is the position in that order", () => {
    const kept: MergeLines = {
      lines: [
        {
          root: "/repo",
          line: [
            { place: 1, branch: "idle", state: "waiting", why: "kept" } as MergeLines["lines"][0]["line"][0],
            { place: 2, branch: "a", state: "gating", batch: "a" },
            { place: 3, branch: "b", state: "gating", batch: "a" },
            { place: 4, branch: "c", state: "waiting" },
          ],
          off: [],
          landed: [],
          sent_back: [],
        },
      ],
    };
    const [one] = views(kept, null);
    expect(one?.line.map((row) => [row.place, row.branch, row.why])).toEqual([
      [1, "a", undefined],
      [2, "b", undefined],
      [3, "idle", "kept"],
      [4, "c", undefined],
    ]);
  });

  test("a landed pull request wears the Job's own badge for how it ended, and one in line its number alone", () => {
    const [one] = views(SERVED, "/repo");
    expect(one?.line.find((row) => row.pr !== undefined)?.pr).toEqual({ number: 1770, url: `${PULL}1770` });
    const closed: MergeLines = {
      lines: [
        {
          root: "/repo",
          line: [],
          off: [],
          landed: [{ branch: "a", state: "landed", pull_request: { number: 2, url: "u", settled: "closed_unmerged" } }],
          sent_back: [],
        },
      ],
    };
    expect(views(closed, null)[0]?.landed[0]?.pr?.settled?.label).toBe("Closed without merging");
  });

  test("a picked repository draws its own line, unnamed, and one without a line draws none", () => {
    const picked = views(SERVED, "/repo");
    expect(picked.map((one) => [one.root, one.name, one.line.length])).toEqual([["/repo", undefined, 6]]);
    expect(views(SERVED, "/elsewhere")).toEqual([]);
  });

  test("nothing draws before Fleet answers", () => {
    expect(views(null, null)).toEqual([]);
  });

  test("a line nobody is in still draws, with what landed and what was sent back", () => {
    const emptied: MergeLines = { lines: [{ ...SERVED.lines[0]!, line: [] }] };
    const [one] = views(emptied, "/repo");
    expect(one?.line).toEqual([]);
    expect(one?.landed).toHaveLength(2);
    expect(one?.sentBack).toHaveLength(2);
    const never: MergeLines = { lines: [{ root: "/repo", line: [], off: [], landed: [], sent_back: [] }] };
    expect(views(never, null)).toEqual([{ root: "/repo", line: [], landed: [], sentBack: [] }]);
  });

  test("All draws one panel per repository with a line, each named by its repository", () => {
    const two: MergeLines = { lines: [SERVED.lines[0]!, { ...SERVED.lines[0]!, root: "/other", line: [] }] };
    expect(views(two, null).map((one) => [one.root, one.name])).toEqual([
      ["/repo", "repo"],
      ["/other", "other"],
    ]);
    expect(views(two, "/other").map((one) => [one.root, one.name])).toEqual([["/other", undefined]]);
  });
});
