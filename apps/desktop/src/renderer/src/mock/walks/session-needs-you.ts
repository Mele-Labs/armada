// What a live Session waits on the person for is the Needs you panel on the composer, the question first and
// the ledger's own rows under it, a glyph an item on its trigger. An item with no act opens a reply box. Over the `session-waiting-on-you` scenario. Told twice: wide, and
// below the breakpoint.

import { button, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";
import { NARROW, toSessions } from "../sessions/walk-kit";

function steps(): Step[] {
  const sessions = region("Work", { exact: true });
  const needs = region("Needs you");
  return [
    { press: inside(sessions, button(/Pin the store clock/)), say: "Opened" },
    { look: needs, say: "Needs you, the panel on the composer, up because a question arrived" },
    { hover: inside(needs, role("img", "Look at the findings page")), say: "One glyph an item, named on hover" },
    { look: inside(needs, button("Open Approve #1843")), say: "The ledger's own rows, each a press that does its act" },
    { press: inside(needs, button("Open Keep the old fixture or drop it?")), say: "An item with no act opens a box" },
    { look: inside(needs, role("textbox", /^Reply to Keep the old fixture/)), say: "What is typed here goes to the agent" },
    { look: role("article", "Waiting on you"), say: "The question is the panel's first part, above the rows" },
  ];
}

const wide = walk("session-waiting-on-you", [toSessions, ...steps()]);
const narrow = walk("session-waiting-on-you", [toSessions, ...steps()], NARROW);

export { wide as "session-needs-you", narrow as "session-needs-you-narrow" };
