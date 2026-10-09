// Approve on the walk window's bar: the person walked the page and it is right. A Session is
// told in a message from the person, which wakes it; a Job gets the press JobDetail's Approve makes.

import { button, dialog, inside, region, role, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const sessionWindow = dialog("Bridge's window on Store clock findings");
const jobWindow = dialog("Bridge's window on mock");

const aSession = walk("session-walk-window", [
  toSessions,
  { press: inside(region("Sessions"), button("Store clock findings")), say: "A Session with nothing shown yet" },
  { type: "Show me what you found\n", into: role("textbox", "Message"), say: "Asked to show it" },
  { look: sessionWindow, say: "The window on what it showed" },
  { press: inside(sessionWindow, button("Approve")), say: "Walked, and it is right" },
  { look: inside(sessionWindow, button("Approved")), say: "The bar keeps it" },
  { look: inside(region("Thread"), text(/^Approved: http/)), say: "The Session is told, with the address, and wakes" },
]);

const aJob = walk("prototype-walked", [
  { look: jobWindow, say: "A Prototype's mock, opened in Bridge's window" },
  { press: inside(jobWindow, button("Approve")), say: "Walked, and it is right: the Job is approved, as from its own Approve" },
  { look: inside(jobWindow, button("Approved")), say: "The window stays open" },
]);

export { aSession as "approving-a-sessions-window", aJob as "approving-a-jobs-walk" };
