// Approved, on the window's bar: once the person approves the page, the bar says so at its start, in the
// success green, and keeps saying it. Over the `walk-variations` scenario, so the take is named.

import { button, dialog, inside, region, role, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const window = dialog("Bridge's window on Board layout takes");
const takes = inside(window, role("radiogroup", "Variations"));

const approved = walk("walk-variations", [
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button("Board layout takes")), say: "A Job with three layouts built" },
  { type: "Show me the layouts\n", into: role("textbox", "Message"), say: "Asked to show them" },
  { press: inside(takes, role("radio", "Needs you first")), say: "B on screen, ready to walk" },
  { press: inside(window, button("Approve B")), say: "Approved" },
  { look: inside(window, text(/^Approved B$/)), say: "The bar says Approved B at its start, in green" },
]);

export { approved as "walk-approved" };
