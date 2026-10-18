// Approve on the walk window's bar: the person walked the page and it is right. A Session is
// told in a message from the person, which wakes it.

import { button, dialog, inside, region, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const sessionWindow = dialog("Bridge's window on Store clock findings");

const aSession = walk("session-walk-window", [
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button("Store clock findings")), say: "A Job with nothing shown yet" },
  { type: "Show me what you found\n", into: role("textbox", "Message"), say: "Asked to show it" },
  { look: sessionWindow, say: "The window on what it showed" },
  { press: inside(sessionWindow, button("Approve")), say: "Walked, and it is right" },
  { look: inside(sessionWindow, button("Approved")), say: "The bar keeps it" },
  { look: inside(region("Thread"), region("You approved")), say: "The Job is told, with the address, and wakes" },
]);

export { aSession as "approving-a-sessions-window" };
