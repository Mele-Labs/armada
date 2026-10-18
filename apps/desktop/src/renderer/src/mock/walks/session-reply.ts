// A waiting item the agent gave no act, in the Needs you band: its glyph is Reply, a press opens a box under the row, a line
// typed and sent settles it and the item leaves the list. Over the `session-reply` scenario. Told
// twice: wide, and below the breakpoint.

import { button, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";
import { NARROW, toSessions } from "../sessions/walk-kit";

function steps(): Step[] {
  const sessions = region("Jobs", { exact: true });
  const mine = inside(sessions, button(/Pin the store clock/));
  const needs = region("Needs you");
  return [
    { press: mine, say: "Opened" },
    { press: inside(needs, button(/^Needs you/)), say: "Needs you, raised from the composer" },
    { hover: inside(needs, role("img", "Reply", { exact: true })), say: "A reply glyph on an item with no act, and on hover its name" },
    { press: inside(needs, button("Open Which date should the clock pin to?")), say: "A press opens a box under the row" },
    { type: "Pin it to the first of March", into: inside(needs, role("textbox", /^Reply to Which date/)), say: "A line typed" },
    { press: inside(needs, button("Send")), say: "Sent" },
    { look: region("Thread"), say: "It reaches the Job as his answer, and the item leaves the list" },
    { press: inside(needs, button("Open Name the helper eventually, or settle?")), say: "An answer item with no choices opens the same box" },
    { type: "Call it settle", into: inside(needs, role("textbox", /^Reply to Name the helper/)), say: "A line typed" },
    { press: inside(needs, button("Send")), say: "Sent, and it leaves the list" },
  ];
}

const wide = walk("session-reply", [toSessions, ...steps()]);
const narrow = walk("session-reply", [toSessions, ...steps()], NARROW);

export { wide as "session-reply", narrow as "session-reply-narrow" };
