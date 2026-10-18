// Whether a walk is ready, on the window's own bar: the Session says it is ready when it shows the
// page, says it is changing it when asked to, and ready again when done. Over `session-walk-window`.

import { button, dialog, inside, region, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const window = dialog("Bridge's window on Store clock findings");

const ready = walk("session-walk-window", [
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button("Store clock findings")), say: "A Job with nothing shown yet" },
  { type: "Show me what you found\n", into: role("textbox", "Message"), say: "Asked to show it" },
  { look: inside(window, role("img", "Ready to walk")), say: "Shown, and ready: the mark and the bar's hue say so" },
  { type: "Put the slowest read first\n", into: role("textbox", "Message"), say: "Asked to change it" },
  { look: inside(window, role("img", "Still changing")), say: "The Job is changing it: the mark moves and the bar takes the running hue" },
  { look: inside(window, button("Not ready")), say: "Approve waits: nothing settled to approve yet" },
  { look: inside(window, role("img", "Ready to walk")), say: "Done, and ready again" },
  { look: inside(window, button("Approve")), say: "Approve is back" },
]);

export { ready as "walk-ready" };
