// The merge line's queue and outcome lists, drawn as a panel under Overview's lists and
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
  { look: inside(ARMADA, role("img", "waiting")), say: "Waiting: its order, the mark, the branch" },
  { look: inside(ARMADA, role("list", "Batch")), say: "Places 1 to 4 gate as one batch" },
  {
    look: inside(ARMADA, role("img", "Preparing to land")),
    say: "Preparing to land: no Check has run yet; hover names it",
  },
  {
    look: inside(ARMADA, role("img", "Running Checks before landing")),
    say: "Running Checks before landing pulses; hover names it",
  },
  {
    look: inside(inside(ARMADA, role("listitem", "worktree-agent-aef3c24792026e2c3")), region("Checks")),
    say: "Its Checks as they run, one segment each",
  },
  { look: inside(ARMADA, text("merging main (c527f60e09)")), say: "Merging main in: words only where no Check runs" },
  { look: inside(ARMADA, role("link", "#1770")), say: "A pull request still in line: its number alone" },
  { look: inside(ARMADA, role("list", "Recently landed")), say: "Recently landed: the newest three" },
  { look: inside(ARMADA, text("007088d7ea")), say: "Each with its merge commit" },
  {
    look: inside(inside(ARMADA, role("listitem", "studio/read-in-lands-in-a-zone")), text("Merged")),
    say: "Its pull request wears the Job's own badge: merged",
  },
  { look: inside(ARMADA, role("list", "Sent back")), say: "Sent back: Checks failed, conflict or stopped" },
  {
    look: inside(inside(ARMADA, role("listitem", "fleet/pulse-log-rows")), region("Checks")),
    say: "Checks failed: the strip opens on the ones that did",
  },
  {
    look: inside(ARMADA, role("listitem", "bridge/overview-strip-width")),
    say: "A conflict whose files wrap: the mark stays on the first line",
  },
  { look: NOTES, say: "A second repository, its own panel" },
  { look: inside(NOTES, role("list", "Recently landed")), say: "Nobody in line, and Recently landed still shows" },
  { look: inside(NOTES, text("Closed without merging")), say: "A pull request the line closed itself, naming the merge" },
  { look: inside(SCRATCH, role("img", "Empty")), say: "Nothing ever landed here: a picture and no words" },
  { press: inside(ARMADA, button("Collapse Merge line, armada")), say: "Each panel folds on its own" },
  { press: inside(ARMADA, button("Expand Merge line, armada")), say: "And opens again" },
  { press: button("Merge line", { exact: true }), say: "Its own row in the rail, under Work" },
  { look: inside(NOTES, role("list", "Recently landed")), say: "The same panels, one per repository" },
  { look: inside(SCRATCH, role("img", "Empty")), say: "The empty line draws here too" },
]);
