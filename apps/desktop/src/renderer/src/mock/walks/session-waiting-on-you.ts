// A Session that waits on the person: its row has a hand, the Needs you panel on its
// composer is up with the question and the items, a press on the page item opens the window, and the question is its first part. Over the `session-waiting-on-you` scenario. Told twice: wide, and
// below the breakpoint.

import { button, dialog, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";
import { NARROW, toSessions } from "../sessions/walk-kit";

function steps(): Step[] {
  const sessions = region("Jobs", { exact: true });
  const needs = region("Needs you");
  const mine = inside(sessions, button(/Pin the store clock/));
  return [
    { hover: inside(sessions, role("img", /^Waiting on you/)), say: "A hand on its row, and on hover what it waits on" },
    { press: mine, say: "Opened" },
    { press: inside(needs, button("Open Look at the findings page")), say: "A page item's row opens the window itself" },
    { look: dialog("Bridge's window on Pin the store clock"), say: "The window" },
    { press: button("Close window"), say: "Closed" },
    { press: inside(needs, button("Open Which clock?")), say: "The question is the first row" },
    { look: role("article", "Waiting on you"), say: "Opened, it is answered in the panel" },
  ];
}

const wide = walk("session-waiting-on-you", [toSessions, ...steps()]);
const narrow = walk("session-waiting-on-you", [toSessions, ...steps()], NARROW);

export { wide as "session-waiting-on-you", narrow as "session-waiting-on-you-narrow" };
