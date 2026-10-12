// A waiting item the agent gave no act: its glyph is Reply, a press opens a box under the row, a line
// typed and sent settles it and the item leaves the list. Over the `session-reply` scenario. Told
// twice: wide, and below the breakpoint.

import { button, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";
import { NARROW, kit, toSessions } from "../sessions/walk-kit";

function steps(narrow: boolean): Step[] {
  const { ledger, opened, row } = kit(narrow);
  const sessions = region("Sessions");
  const mine = inside(sessions, button(/Pin the store clock/));
  const waiting = inside(ledger, region("Waiting on you"));
  return [
    { press: mine, say: "Opened" },
    ...opened([{ hover: inside(waiting, role("img", "Reply", { exact: true })), say: "A reply glyph on an item with no act, and on hover its name" }]),
    ...row("Open Which date should the clock pin to?", "A press opens a box under the row"),
    { type: "Pin it to the first of March", into: inside(ledger, role("textbox", /^Reply to Which date/)), say: "A line typed" },
    { press: inside(ledger, button("Send")), say: "Sent" },
    { look: region("Thread"), say: "It reaches the Session as his answer, and the item leaves the list" },
    ...(narrow
      ? []
      : [
          ...row("Open Name the helper eventually, or settle?", "An answer item with no choices opens the same box"),
          { type: "Call it settle", into: inside(ledger, role("textbox", /^Reply to Name the helper/)), say: "A line typed" },
          { press: inside(ledger, button("Send")), say: "Sent, and it leaves the list" },
        ]),
  ];
}

const wide = walk("session-reply", [toSessions, ...steps(false)]);
const narrow = walk("session-reply", [toSessions, ...steps(true)], NARROW);

export { wide as "session-reply", narrow as "session-reply-narrow" };
