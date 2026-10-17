// What a live Session waits on the person for hangs on the composer as a tab of one
// glyph an item: a press raises a sheet with the ledger's own rows, an item with no act opens a reply box, and
// the question keeps its own card. Over the `session-waiting-on-you` scenario. Told twice: wide, and
// below the breakpoint.

import { button, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";
import { NARROW, toSessions } from "../sessions/walk-kit";

function steps(): Step[] {
  const sessions = region("Work", { exact: true });
  const needs = region("Needs you");
  return [
    { press: inside(sessions, button(/Pin the store clock/)), say: "Opened" },
    { look: needs, say: "Needs you, a tab on the composer" },
    { hover: inside(needs, role("img", "Look at the findings page")), say: "One glyph an item, named on hover" },
    { press: inside(needs, button(/^Needs you/)), say: "A press raises the sheet" },
    { look: inside(needs, button("Open Approve #1843")), say: "The ledger's own rows, each a press that does its act" },
    { press: inside(needs, button("Open Keep the old fixture or drop it?")), say: "An item with no act opens a box" },
    { look: inside(needs, role("textbox", /^Reply to Keep the old fixture/)), say: "What is typed here goes to the agent" },
    { look: role("article", "Waiting on you"), say: "The question keeps its own card, and is not in the sheet" },
  ];
}

const wide = walk("session-waiting-on-you", [toSessions, ...steps()]);
const narrow = walk("session-waiting-on-you", [toSessions, ...steps()], NARROW);

export { wide as "session-needs-you", narrow as "session-needs-you-narrow" };
