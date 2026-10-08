// A Session that is quiet or ended, forked. One from a terminal whose mod stopped asking has its conversation
// and a Fork in its head, and no message box; a live one has the box and no Fork. Fork starts a new
// Session with the same title and the old conversation, which opens at once with a row linking back,
// and the old one now links forward. Told twice, wide and below the breakpoint.

import { kit, NARROW, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

function steps(narrow: boolean): Step[] {
  const { message, thread, opened, row } = kit(narrow);
  const sessions = region("Sessions");
  return [
    { look: inside(sessions, button("Fix the flaky store test")), say: "A live Session is in the list" },
    { press: inside(sessions, button("Fix the flaky store test")), say: "It opens" },
    { look: message, say: "It takes a message" },
    { press: button("Sessions"), say: "Back to the list" },
    { press: inside(sessions, role("tab", "Quiet")), say: "Quiet is a view of its own" },
    { look: inside(sessions, role("heading", "Quiet")), say: "A terminal Session whose mod is not asking is under Quiet" },
    { press: inside(sessions, role("tab", "Ended")), say: "So is Ended" },
    { look: inside(sessions, role("heading", "Ended")), say: "One that ended is under Ended, apart from it" },
    { press: inside(sessions, role("tab", "Quiet")), say: "Back to Quiet" },
    { press: inside(sessions, button("Why the reader drops lines")), say: "A terminal Session whose mod stopped asking" },
    { look: inside(thread, text("It splits on newlines")), say: "Its conversation is still there" },
    { look: button("Fork"), say: "In place of the message box, a Fork" },
    { press: button("Fork"), say: "A new Session starts with this conversation" },
    { look: region("Session s-01FORK00"), say: "It opens at once, with the old title" },
    { look: role("group", "Forked from Why the reader drops lines"), say: "Its thread opens on one quiet row, closed" },
    { press: role("button", "Forked from Why the reader drops lines", { exact: true }), say: "Pressed, it opens onto the old conversation" },
    { look: inside(role("group", "Forked from Why the reader drops lines"), text("It splits on newlines")), say: "What was said before the fork, to read" },
    ...opened([{ look: inside(role("region", "Forks"), text("Forked from Why the reader drops lines")), say: "Its ledger says where it came from" }]),
    { look: message, say: "And it takes a message" },
    ...row("Open Forked from Why the reader drops lines", "The row opens the Session it came from"),
    { look: button("Fork"), say: "Still dead, and still offers Fork" },
    ...opened([{ look: inside(role("region", "Forks"), text("Forked to Why the reader drops lines")), say: "Its ledger now points forward" }]),
  ];
}

const wide = walk("session-fork", [toSessions, ...steps(false)]);
const narrow = walk("session-fork", [toSessions, ...steps(true)], NARROW);

export { wide as "session-fork", narrow as "session-fork-narrow" };
