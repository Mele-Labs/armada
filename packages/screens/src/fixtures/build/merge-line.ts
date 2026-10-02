// The merge line as `armada land --status` printed it on 2 Oct 2026, and the
// three outcomes a branch leaves the line with.
//
// **The line is that run, row for row.** Six branches: the first and the last
// waiting, and four in one turn between them. The runner had told two of the
// four it was reading `verify-foundations` and the other two it was merging
// main in, so the details differ inside one batch, as they did. `--status`
// prints a waiting row's detail as `in line`, which the mark already says, so
// none is carried.
//
// **What left the line is made up from real branches.** `bridge/land-board-reads-plainly`
// did land, at `29064cc27a`; the red and the conflict are invented, on
// Checks and files this repository has.

import type { MergeLineView } from "../../draft/merge-line";

const PULL = "https://git.example/armada/pull/";

/** The batch the turn was gating, by the first member's branch. */
const TURN = "docs/wire-lock-signed";

export function mergeLine(): MergeLineView {
  return {
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
        doing: "reading verify-foundations against main",
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
        pr: { number: 1770, url: `${PULL}1770` },
        state: "waiting",
      },
    ],
    off: [
      { branch: "bridge/land-board-reads-plainly", state: "landed", merge: "29064cc27a" },
      {
        branch: "fleet/pulse-log-rows",
        pr: { number: 1768, url: `${PULL}1768` },
        state: "red",
        failed: ["desktop_test", "screens_test"],
      },
      {
        branch: "bridge/overview-strip-width",
        state: "conflict",
        conflicts: ["apps/desktop/src/renderer/src/App.tsx", "packages/screens/src/OverviewLists.tsx"],
      },
    ],
  };
}
