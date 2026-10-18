// A Session's pull request turns main red. The head names the Session beside the pull request that
// merged, and Send back lists Sessions with the Session already chosen, the brief there to
// edit as the message it is sent as.

import { button, dialog, inside, region, role, text, viaPalette, walk } from "../walk";

const MERGE = region("Merge line");
const RED = inside(MERGE, role("status", "Main is red"));
const SEND = dialog("Send the work back");
const PULLS = inside(MERGE, role("list", "Open pull requests"));
const LANDED = inside(MERGE, role("list", "Recently landed"));

const SESSION = "Show who merged on the head";

export const mainRedSession = walk("main-red-session", [
  ...viaPalette("Merge line", "The merge line"),
  { look: inside(MERGE, role("img", "Main is green")), say: "Main is green" },
  { later: inside(MERGE, role("img", "Main is green")), say: "A Job's pull request merges" },
  { look: RED, say: "Main is red" },
  { look: inside(RED, role("link", "#1830")), say: "Broke in: the pull request that merged" },
  { look: inside(RED, text("armada/session-c279e883")), say: "And its branch" },
  { hover: inside(RED, role("img", "Job")), say: "Whose it was: a Job, by its icon" },
  { look: inside(RED, button(SESSION)), say: "Its title opens the Job" },
  { look: inside(LANDED, role("listitem", /^armada\/session-c279e883, landed/)), say: "In Recently landed: the same pull request, a red run on main, and the Job that opened it" },
  { look: inside(LANDED, role("img", "ci failed on main")), say: "The run on its merge commit failed" },
  { look: inside(RED, button("Dispatch a new Job")), say: "And dispatching a new Job, in the band" },
  { look: inside(RED, button("Send back", { exact: true })), say: "Send back sits beside the Job that merged: it goes back to that one" },
  { look: inside(PULLS, role("listitem", /^armada\/14-wire-the-cockpit/)), say: "A long branch is cut short, so the number and its owner stay inside the row" },
  { hover: inside(PULLS, button(/^Wire the Cockpit/)), say: "The owner is cut short too: hover has its whole title" },
  { press: inside(RED, button("Send back", { exact: true })), say: "Send back" },
  { look: inside(SEND, role("list", "Attached")), say: "What goes with it" },
  { look: inside(SEND, role("radiogroup", "Jobs")), say: "The Jobs it can go back to" },
  { look: inside(SEND, role("img", "Merged #1830")), say: "The Job that merged comes first in its group, marked" },
  { look: inside(SEND, role("radio", new RegExp(`^${SESSION}`))), say: "And is already chosen" },
  { look: inside(SEND, role("textbox", "Message")), say: "The brief, there to edit: it goes to the Job as a message" },
  { press: inside(SEND, button("Send back", { exact: true })), say: "Send it" },
  { later: inside(RED, button("Send back", { exact: true })), say: "The Job fixes it" },
  { look: inside(MERGE, role("img", "Main is green")), say: "Main is green again" },
  { look: inside(LANDED, role("listitem", /^armada\/session-c279e883-fix, landed/)), say: "The Job's fix is at the top of Recently landed, its run green" },
]);
