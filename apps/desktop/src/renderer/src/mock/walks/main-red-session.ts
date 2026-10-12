// A Session's pull request turns main red. The head names the Session beside the pull request that
// merged, and Send back lists Sessions and Jobs with the Session already chosen, the brief there to
// edit as the message it is sent as.

import { button, dialog, inside, region, role, text, walk } from "../walk";

const MERGE = region("Merge line");
const RED = inside(MERGE, role("status", "Main is red"));
const SEND = dialog("Send the work back");

const SESSION = "Show who merged on the head";

export const mainRedSession = walk("main-red-session", [
  { press: role("button", "Merge line", { exact: true }), say: "The merge line" },
  { look: inside(MERGE, role("img", "Main is green")), say: "Main is green" },
  { later: inside(MERGE, role("img", "Main is green")), say: "A Session's pull request merges" },
  { look: RED, say: "Main is red" },
  { look: inside(RED, role("link", "#1830")), say: "Broke in: the pull request that merged" },
  { look: inside(RED, text("armada/session-c279e883")), say: "And its branch" },
  { hover: inside(RED, role("img", "Session")), say: "Whose it was: a Session, by its icon" },
  { look: inside(RED, button(SESSION)), say: "Its title opens the Session" },
  { look: inside(RED, button("Dispatch a new Job")), say: "Two acts" },
  { hover: inside(RED, button(/^Send back to /)), say: "Send back names who it goes back to: the Session that merged" },
  { press: inside(RED, button(/^Send back to /)), say: "Send back" },
  { look: inside(SEND, role("list", "Attached")), say: "What goes with it" },
  { look: inside(SEND, role("radiogroup", "Sessions")), say: "Sessions first, then Jobs" },
  { look: inside(SEND, role("img", "Merged #1830")), say: "The Session that merged comes first in its group, marked" },
  { look: inside(SEND, role("radio", new RegExp(`^${SESSION}`))), say: "And is already chosen" },
  { look: inside(SEND, role("radiogroup", "Jobs")), say: "Jobs below" },
  { look: inside(SEND, role("textbox", "Message")), say: "The brief, there to edit: it goes to the Session as a message" },
  { press: inside(SEND, button("Send back", { exact: true })), say: "Send it" },
  { later: inside(RED, button(/^Send back to /)), say: "The Session fixes it" },
  { look: inside(MERGE, role("img", "Main is green")), say: "Main is green again" },
]);
