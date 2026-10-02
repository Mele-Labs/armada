// A Check's log, opened from the Checks strip wherever it is drawn (owner, 2 Oct 2026): a plan
// group's boundary and a merge line's turn. Each segment and each row is one Check; pressing one
// opens its log in the log panel, following it while the Check runs and readable once it has ended.

import { button, card, dialog, inside, region, role, tab, text, walk } from "../walk";

const BOUNDARY = region("Checks at this boundary");
const LOG = dialog("Check log");
const TURN = role("listitem", "worktree-agent-aef3c24792026e2c3");
const SENT_BACK = role("listitem", "fleet/pulse-log-rows");

export const aChecksLogFromItsStrip = walk("check-logs", [
  { press: tab("Plan"), say: "Group 3's Checks are running" },
  { press: card("Group 3"), say: "Its panel draws the boundary's Checks, a segment each" },
  { hover: inside(BOUNDARY, button("screens_test, running")), say: "Hover names the Check" },
  { press: inside(BOUNDARY, button("screens_test, running")), say: "Pressing it opens the Check's log" },
  { look: inside(LOG, role("img", "Being written")), say: "Still being written: the mark pulses" },
  { look: inside(LOG, text("running.test.tsx")), say: "A new line arrives as the Check prints it" },
  { press: inside(LOG, button("Close")), say: "Close, or Esc, puts it away" },
  { press: inside(BOUNDARY, button("Checks", { exact: true })), say: "The strip opens to one row per Check" },
  { look: inside(BOUNDARY, text("bridge_build")), say: "A Check not started yet is no button" },
  { press: inside(BOUNDARY, button("test, passed", { exact: true })), say: "A Check that has ended opens too" },
  { look: inside(LOG, text("4154 passed")), say: "Its whole log, with no mark" },
  { press: inside(LOG, button("Close")), say: "Back to the group" },
  { press: button("Merge line", { exact: true }), say: "The merge line draws the same strip" },
  { press: inside(TURN, button("screens_test, running")), say: "A turn's running Check opens the same panel" },
  { look: inside(LOG, text("Row.test.ts")), say: "Its log grows as the runner writes it" },
  { press: inside(LOG, button("Close")), say: "Put it away" },
  { press: inside(SENT_BACK, button("desktop_test, failed")), say: "A Check that failed, sent back" },
  { look: inside(LOG, text("AssertionError")), say: "Its whole log, readable after it ended" },
]);
