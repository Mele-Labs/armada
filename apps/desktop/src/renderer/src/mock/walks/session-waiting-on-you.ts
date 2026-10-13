// A Session that waits on the person: it sits under Needs you with a hand, the band at the foot of its
// thread opens to the items, a press on the page item opens the window, and the question is on its own
// card in the thread. Over the `session-waiting-on-you` scenario. Told twice: wide, and
// below the breakpoint.

import { button, dialog, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";
import { NARROW, toSessions } from "../sessions/walk-kit";

function steps(): Step[] {
  const sessions = region("Sessions");
  const needs = region("Needs you");
  const mine = inside(sessions, button(/Pin the store clock/));
  return [
    { look: inside(sessions, text("Needs you")), say: "The Session sits under Needs you" },
    { hover: inside(sessions, role("img", /^Waiting on you:/)), say: "A hand, and on hover what it waits on" },
    { press: mine, say: "Opened" },
    { press: inside(needs, button(/^Needs you/)), say: "Needs you, opened at the foot of the thread" },
    { press: inside(needs, button("Open Look at the findings page")), say: "The page item opens the window" },
    { look: dialog("Bridge's window on Pin the store clock"), say: "The window" },
    { press: button("Close window"), say: "Closed" },
    { look: role("article", "Waiting on you"), say: "The question is on its own card, and not in the band" },
  ];
}

const wide = walk("session-waiting-on-you", [toSessions, ...steps()]);
const narrow = walk("session-waiting-on-you", [toSessions, ...steps()], NARROW);

export { wide as "session-waiting-on-you", narrow as "session-waiting-on-you-narrow" };
