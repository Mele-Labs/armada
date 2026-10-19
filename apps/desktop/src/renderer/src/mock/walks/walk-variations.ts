// Takes of one change in one window: the bar swaps between them, a note names the take it is on,
// and Approve approves the take on screen. Over the `walk-variations` scenario.

import { button, dialog, inside, region, role, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const window = dialog("Bridge's window on Board layout takes");
const takes = inside(window, role("radiogroup", "Variations"));

const variations = walk("walk-variations", [
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button("Board layout takes")), say: "A Job with three layouts built" },
  { type: "Show me the layouts\n", into: role("textbox", "Message"), say: "Asked to show them" },
  { look: takes, say: "One window, three takes on its bar; A is on screen" },
  { press: inside(takes, role("radio", "Needs you first")), say: "B swaps in, in the same window" },
  { press: inside(takes, role("radio", "Sessions beside")), say: "And C" },
  { press: inside(takes, role("radio", "Needs you first")), say: "Back to B" },
  { press: inside(window, button("Capture")), say: "A note taken now is on B: the note's card says so, and the Job is told which take" },
  { press: inside(window, button("Approve B")), say: "Approve names the take on screen" },
  { look: inside(region("Thread"), text(/^Approved B, Needs you first: http/)), say: "The Job is told which take, with its address" },
  { press: inside(takes, role("radio", "Sessions beside")), say: "On another take, Approve is offered again; pressing it moves the approval" },
]);

export { variations as "walk-variations" };
