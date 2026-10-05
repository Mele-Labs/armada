// A Check fails while a merge-line turn runs on, and the owner is told then, not when the turn ends.
// Three lines, each at its own place in the story: a batch told once before the split names the
// one at fault, a single branch told at once, and a Check red on main too. The Job's own log has
// the failure as a line. Each `later` is time passing: the scenario publishes its next moment.

import { button, dialog, inside, region, role, tab, text, walk } from "../walk";

const ARMADA = region("Merge line, armada");
const NOTES = region("Merge line, notes");
const SCRATCH = region("Merge line, scratch");
const BRIDGE = region("Merge line, bridge");
const ARMADA_ALERT = inside(ARMADA, role("status"));
const NOTES_ALERT = inside(NOTES, role("status"));
const CARRIER = role("listitem", "worktree-agent-aef3c24792026e2c3");

export const theFailedCheckMidTurn = walk("merge-line-failed-check", [
  { look: inside(ARMADA, role("list", "Batch")), say: "A batch of three, one turn" },
  { press: inside(ARMADA, button(/^Open guide 22/)), say: "The ? beside the heading opens the guide" },
  { look: dialog(/^Guide 22/), say: "What the line is, what the number and bracket mean, and each waiting mark" },
  { press: inside(dialog(/^Guide 22/), button("Close")), say: "Put the guide away" },
  { later: inside(CARRIER, button("screens_test, running")), say: "screens_test running, the rest waiting" },
  { look: inside(CARRIER, button("screens_test, failed")), say: "screens_test failed and its rerun failed: desktop_test runs on behind it" },
  { look: inside(ARMADA, role("status")), say: "One alert for the batch, before the split names the branch at fault" },
  { look: inside(ARMADA_ALERT, text("armada")), say: "It says which repository it is in: there is more than one" },
  { look: inside(ARMADA_ALERT, text("docs/wire-lock-signed")), say: "Every branch the Check failed for: the batch's first member" },
  { look: inside(ARMADA_ALERT, text("worktree-agent-aef3c24792026e2c3")), say: "The second member" },
  { look: inside(ARMADA_ALERT, text("fleet/gate-policy-every-run")), say: "The third" },
  { look: inside(ARMADA_ALERT, button("Open log")), say: "Open log is a button, filled and bordered" },
  { look: inside(NOTES, role("status")), say: "One branch: told at once" },
  { look: inside(NOTES_ALERT, text("notes/reading-list-tags")), say: "A single branch is named too, batched or not" },
  { look: inside(SCRATCH, role("status")), say: "Another branch, told the same way" },
  { press: text("Cache the manifest read between dispatches"), say: "A Job in the batch" },
  { press: tab("Pulse"), say: "Its Pulse" },
  { press: button(/Job log/), say: "Its own log" },
  { later: text("screens_test failed in the merge line turn"), say: "The failure is a line in the Job's own log" },
  { look: text("screens_test is fleet/gate-policy-every-run's"), say: "The split names the branch; this Job is cleared" },
  { press: button(/^Close/), say: "Back to the Job" },
  { press: button("Overview", { exact: true }), say: "Back to the lines" },
  { look: inside(ARMADA, role("status")), say: "The same alert now names the branch" },
  { later: inside(SCRATCH, text("ports_test red on main")), say: "Red on main too: nobody's, and the turn holds" },
  { look: inside(ARMADA, role("list", "Sent back")), say: "The turn ends and the branch is sent back" },
  { look: inside(ARMADA, text("sent back")), say: "The alert became the verdict, in the same place" },
  { look: inside(NOTES, role("list", "Sent back")), say: "The single branch, sent back" },
  { look: inside(BRIDGE, role("listitem", "canvas/gates-on-the-spine")), say: "Ordered by what merges next: the running turn first, from 1" },
  { look: inside(BRIDGE, role("listitem", "fix/overview-fallback-and-reads")), say: "Its second member, 2" },
  { hover: inside(BRIDGE, role("img", /Kept its place/)), say: "Next up, and it kept its place after a red" },
  { look: inside(BRIDGE, role("listitem", "fix/dispatch-did-not-answer")), say: "Queued behind: no reason, no mark" },
  { hover: inside(BRIDGE, role("img", /clashes with another branch/)), say: "Left out of the turn: clashes with a member" },
  { hover: inside(BRIDGE, role("img", /clashes with main/)), say: "Left out of the turn: clashes with main" },
  { hover: inside(BRIDGE, role("img", /Joined after/)), say: "Joined after the turn began" },
]);
