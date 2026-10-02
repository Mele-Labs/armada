// The merge line: what `armada land --status` prints, drawn as a panel under Overview's lists and
// again on a rail surface of its own. The batch a turn gates is one group, each state is a mark
// with its word on hover, and what landed and what was sent back read below it in two lists. On
// All, each repository Fleet serves a line for has its own panel, named by its repository.

import { button, inside, region, role, text, walk } from "../walk";

const ARMADA = region("Merge line, armada");
const NOTES = region("Merge line, notes");
const SCRATCH = region("Merge line, scratch");

export const theMergeLine = walk("merge-line", [
  { look: ARMADA, say: "On All, one panel for each repository with a line" },
  { look: inside(ARMADA, text("armada")), say: "Named by its repository" },
  { look: inside(ARMADA, role("img", "waiting")), say: "Waiting: its place, the mark, the branch" },
  { look: inside(ARMADA, role("list", "Batch")), say: "Places 2 to 5 gate as one batch" },
  { look: inside(ARMADA, role("img", "gating")), say: "Gating pulses; hover names the state" },
  { look: inside(ARMADA, text("merging main (c527f60e09)")), say: "What the runner is doing now" },
  { look: inside(ARMADA, role("link", "#1770")), say: "Its pull request" },
  { look: inside(ARMADA, role("list", "Recently landed")), say: "Recently landed: the newest three" },
  { look: inside(ARMADA, text("007088d7ea")), say: "Each with its merge commit" },
  { look: inside(ARMADA, role("list", "Sent back")), say: "Sent back: red, conflict or stopped" },
  { look: inside(ARMADA, text("desktop_test")), say: "Red, with the Checks that failed" },
  {
    look: inside(ARMADA, role("listitem", "bridge/overview-strip-width")),
    say: "Conflict, with the files that did not merge",
  },
  { look: NOTES, say: "A second repository, its own panel" },
  { look: inside(NOTES, role("list", "Recently landed")), say: "Nobody in line, and Recently landed still shows" },
  { look: inside(SCRATCH, role("img", "Empty")), say: "Nothing ever landed here: a picture and no words" },
  { press: inside(ARMADA, button("Collapse Merge line, armada")), say: "Each panel folds on its own" },
  { press: inside(ARMADA, button("Expand Merge line, armada")), say: "And opens again" },
  { press: button("Merge line", { exact: true }), say: "Its own row in the rail, under Work" },
  { look: inside(NOTES, role("list", "Recently landed")), say: "The same panels, one per repository" },
  { look: inside(SCRATCH, role("img", "Empty")), say: "The empty line draws here too" },
]);
