// A Session that waits on the person: it sits under Needs you with a hand, its ledger opens on a
// Waiting on you section, a press on the page item opens the window and a press on the question item
// goes to the card in the thread. Over the `session-waiting-on-you` scenario. Told twice: wide, and
// below the breakpoint.

import { button, dialog, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";
import { NARROW, kit, toSessions } from "../sessions/walk-kit";

function steps(narrow: boolean): Step[] {
  const { ledger, opened, row } = kit(narrow);
  const sessions = region("Sessions");
  const mine = inside(sessions, button(/Pin the store clock/));
  return [
    { look: inside(sessions, text("Needs you")), say: "The Session sits under Needs you" },
    { hover: inside(sessions, role("img", /^Waiting on you:/)), say: "A hand, and on hover what it waits on" },
    { press: mine, say: "Opened" },
    ...opened([
      { look: inside(ledger, region("Waiting on you")), say: "Waiting on you, first in the ledger" },
      { look: inside(ledger, button("Choice 1 Fake")), say: "A question with choices has them numbered" },
    ]),
    ...row("Open Look at the findings page", "The page item opens the window"),
    { look: dialog("Bridge's window on Pin the store clock"), say: "The window" },
    { press: button("Close window"), say: "Closed" },
    ...row("Open Which clock?", "The question item goes to the card in the thread"),
    { look: role("article", "Waiting on you"), say: "The card" },
  ];
}

const wide = walk("session-waiting-on-you", [toSessions, ...steps(false)]);
const narrow = walk("session-waiting-on-you", [toSessions, ...steps(true)], NARROW);

export { wide as "session-waiting-on-you", narrow as "session-waiting-on-you-narrow" };
