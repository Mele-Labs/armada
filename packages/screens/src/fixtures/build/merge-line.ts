// The merge line `armada land --status` printed on 2 Oct 2026, what landed and
// what was sent back, as Fleet serves it, so the mock draws through the same
// fold a real Bridge does (`../../merge-line`). Beside it, a line nobody is in,
// and one nothing has ever landed in.
//
// **The line is that run, row for row.** Six branches: the first and the last
// waiting, four in one turn between them, told different things as they were:
// one reads `verify-foundations`, one is in its Checks, two merge main in.
// Fleet carries no detail for a waiting row; the mark says it.
//
// **What left the line is made up from real branches.** The three landings
// did land, at the commits shown; the red and the conflict are invented.

import type { MergeLine, MergeLines } from "@armada/protocol";

import { repository } from "./base";

const PULL = "https://git.example/armada/pull/";

/** The batch the turn was gating, by the first member's branch. */
const TURN = "docs/wire-lock-signed";

export function mergeLines(): MergeLines {
  return {
    lines: [
      {
        root: repository().root,
        line: [
          { place: 1, branch: "fleet/helm-kills-processes", state: "waiting" },
          {
            place: 2,
            branch: "docs/wire-lock-signed",
            state: "gating",
            batch: TURN,
            doing: "reading verify-foundations against main",
          },
          {
            place: 3,
            branch: "worktree-agent-aef3c24792026e2c3",
            state: "gating",
            batch: TURN,
            checks: [
              { name: "build", state: "passed" },
              { name: "typecheck", state: "passed" },
              { name: "screens_test", state: "running" },
              { name: "desktop_test", state: "waiting" },
              { name: "components_test", state: "waiting" },
            ],
          },
          {
            place: 4,
            branch: "worktree-agent-a0087811ec86c6d80",
            state: "gating",
            batch: TURN,
            doing: "merging main (c527f60e09) into fleet/gate-policy-every-run",
          },
          {
            place: 5,
            branch: "fleet/gate-policy-every-run",
            state: "gating",
            batch: TURN,
            doing: "merging main (c527f60e09) into fleet/gate-policy-every-run",
          },
          {
            place: 6,
            branch: "fleet/read-in-cluster-membership",
            pull_request: { number: 1770, url: `${PULL}1770` },
            state: "waiting",
          },
        ],
        off: [],
        landed: [
          {
            branch: "studio/read-in-lands-in-a-zone",
            pull_request: { number: 1772, url: `${PULL}1772` },
            state: "landed",
            merge_commit: "007088d7ea0a2e909d3ebb072671e978f79ff9a0",
          },
          {
            branch: "bridge/remove-log-leftovers",
            pull_request: { number: 1773, url: `${PULL}1773` },
            state: "landed",
            merge_commit: "3fa640ad4832c11648974d5580862d5154a27a93",
          },
          {
            branch: "bridge/land-board-reads-plainly",
            state: "landed",
            merge_commit: "29064cc27aaf70cbbb924eee1d2212693540c9ad",
          },
        ],
        sent_back: [
          {
            branch: "fleet/pulse-log-rows",
            pull_request: { number: 1768, url: `${PULL}1768` },
            state: "red",
            failed: ["desktop_test", "screens_test"],
            checks: [
              { name: "build", state: "passed" },
              { name: "typecheck", state: "passed" },
              { name: "desktop_test", state: "failed" },
              { name: "screens_test", state: "timed_out" },
            ],
          },
          {
            branch: "bridge/overview-strip-width",
            state: "conflict",
            // Enough files that the detail wraps: the row's mark stays on its first line.
            conflicts: [
              "apps/desktop/src/renderer/src/App.tsx",
              "packages/screens/src/OverviewLists.tsx",
              "packages/screens/src/OverviewSummary.tsx",
              "packages/components/src/compositions/TheShell/TheShell.tsx",
              "packages/components/src/compositions/TheShell/TheShell.css",
            ],
          },
        ],
      },
    ],
  };
}

/** A line nobody is in, in `root`: two landed, and nothing sent back. */
export function emptiedLine(root: string): MergeLine {
  return {
    root,
    line: [],
    off: [],
    landed: [
      { branch: "notes/weekly-review", state: "landed", merge_commit: "a41c9e07d2b85f3e61c0aa9d2f7b4e8c19d03f5a" },
      { branch: "notes/reading-list", state: "landed", merge_commit: "6be2f0c4d18a93e57f0b2c6d4a8e1f3b97c5d20e" },
    ],
    sent_back: [],
  };
}

/** A line in `root` that nothing has ever landed in, and nobody is in or was sent back from. */
export function neverLanded(root: string): MergeLine {
  return { root, line: [], off: [], landed: [], sent_back: [] };
}
