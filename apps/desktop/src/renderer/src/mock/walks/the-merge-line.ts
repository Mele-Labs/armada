// The merge line: what `armada land --status` prints, drawn as a panel under Overview's lists and
// again on a rail surface of its own. The batch a turn gates is one group, each state is a mark
// with its word on hover, and the three outcomes a branch leaves the line with read below it. The
// two conflicts carry the two candidate glyphs for that state, one each.

import { button, inside, region, role, text, walk } from "../walk";

const LINE = region("Merge line");

export const theMergeLine = walk("merge-line", [
  { look: LINE, say: "The merge line, under Overview's lists" },
  { look: inside(LINE, role("img", "waiting")), say: "Waiting: its place, the mark, the branch" },
  { look: inside(LINE, role("list", "Batch")), say: "Places 2 to 5 gate as one batch" },
  { look: inside(LINE, role("img", "gating")), say: "Gating pulses; hover names the state" },
  { look: inside(LINE, text("merging main (c527f60e09)")), say: "What the runner is doing now" },
  { look: inside(LINE, role("link", "#1770")), say: "Its pull request" },
  { look: inside(LINE, role("list", "Left the line")), say: "Off the line: landed, red, conflict" },
  { look: inside(LINE, text("29064cc27a")), say: "Landed, with its merge commit" },
  { look: inside(LINE, text("desktop_test")), say: "Red, with the Checks that failed" },
  { look: inside(LINE, role("listitem", "bridge/overview-strip-width")), say: "Conflict candidate 1: unplug" },
  { look: inside(LINE, role("listitem", "fleet/land-status-json")), say: "Conflict candidate 2: wrench" },
  { press: inside(LINE, button("Collapse Merge line")), say: "It folds like the panels above it" },
  { press: inside(LINE, button("Expand Merge line")), say: "And opens again" },
  { press: button("Merge line", { exact: true }), say: "Its own row in the rail, under Work" },
  { look: inside(LINE, role("link", "#1770")), say: "The same panel: the same rows and the same link" },
]);
