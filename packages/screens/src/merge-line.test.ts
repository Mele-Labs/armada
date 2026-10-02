// The merge line folded from what Fleet serves onto the panel's rows.
//
// `merge-lines.served.json` is the answer `crates/fleet/src/tests/merge_lines.rs` reads back off a
// real `armada-land/` directory and holds equal to this file, so this is the other half of one
// reading of the wire.

import type { MergeLines } from "@armada/protocol";
import { describe, expect, test } from "vitest";

import served from "./fixtures/build/merge-lines.served.json";
import { mergeLineView } from "./merge-line";

const SERVED = served as MergeLines;
const PULL = "https://git.example/armada-dev/armada/pull/";

describe("the merge line Fleet serves", () => {
  test("folds onto the panel's rows: places, the batch, the runner's words, and what left", () => {
    expect(mergeLineView(SERVED, null)).toEqual({
      line: [
        { branch: "fleet/helm-kills-processes", place: 1, state: "waiting" },
        {
          branch: "docs/wire-lock-signed",
          place: 2,
          state: "gating",
          doing: "reading verify-foundations against main",
          batch: "docs/wire-lock-signed",
        },
        {
          branch: "worktree-agent-a",
          place: 3,
          state: "gating",
          doing: "reading verify-foundations against main",
          batch: "docs/wire-lock-signed",
        },
        {
          branch: "fleet/gate-policy-every-run",
          place: 4,
          state: "gating",
          doing: "merging main (c527f60e09) into fleet/gate-policy-every-run",
          batch: "docs/wire-lock-signed",
        },
        { branch: "fleet/push-the-base", place: 5, state: "merging", doing: "pushing the merge onto main" },
        {
          branch: "fleet/read-in-cluster-membership",
          place: 6,
          pr: { number: 1770, url: `${PULL}1770` },
          state: "waiting",
        },
      ],
      off: [
        { branch: "bridge/overview-strip-width", state: "conflict", conflicts: ["apps/desktop/src/renderer/src/App.tsx"] },
        {
          branch: "fleet/pulse-log-rows",
          pr: { number: 1768, url: `${PULL}1768` },
          state: "red",
          failed: ["desktop_test", "screens_test"],
        },
        { branch: "bridge/land-board-reads-plainly", state: "landed", merge: "29064cc27a" },
      ],
    });
  });

  test("a picked repository draws its own line, and one without a line draws none", () => {
    expect(mergeLineView(SERVED, "/repo")?.line).toHaveLength(6);
    expect(mergeLineView(SERVED, "/elsewhere")).toBeUndefined();
  });

  test("nothing draws before Fleet answers, or with nobody in line", () => {
    expect(mergeLineView(null, null)).toBeUndefined();
    const emptied: MergeLines = { lines: [{ ...SERVED.lines[0]!, line: [] }] };
    expect(mergeLineView(emptied, null)).toBeUndefined();
    expect(mergeLineView(emptied, "/repo")).toBeUndefined();
  });

  test("All draws none where two repositories each have a line, rather than one unnamed", () => {
    const two: MergeLines = { lines: [SERVED.lines[0]!, { ...SERVED.lines[0]!, root: "/other" }] };
    expect(mergeLineView(two, null)).toBeUndefined();
    expect(mergeLineView(two, "/other")?.line).toHaveLength(6);
  });
});
