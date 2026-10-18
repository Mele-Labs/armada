// The failure toast's Dispatch job, the face of a split control with Copy debug info, File an issue and Reload Bridge behind its
// caret. A crash in the Sessions store raises the toast; one press starts a Session in Armada's repository whose first message is the
// debug info, opens it, and puts the toast away. Over `failure-dispatch-session`.

import { kit } from "../sessions/walk-kit";
import { button, dialog, inside, role, tab, text, walk } from "../walk";

const { thread } = kit(false);
const request = inside(dialog("Dispatch"), role("textbox", "Request"));

const failureDispatchSession = walk("failure-dispatch-session", [
  { press: inside(role("region", "Dispatch controls"), button("Dispatch")), say: "Dispatch" },
  { press: inside(dialog("Dispatch"), tab("Job")), say: "Job" },
  { type: "Look into the flaky store test", into: request, say: "A request for a new Job" },
  { press: role("radio", "Repository armada"), say: "In armada" },
  { key: "Enter", on: request, say: "A new Job, and the Jobs store throws" },
  { look: button("Dispatch job"), say: "The failure stands over the window. The face of its one control is Dispatch job, with Dismiss beside it" },
  { press: button("More ways to act on this failure"), say: "The caret opens the rest" },
  { look: role("menuitem", "Copy debug info"), say: "Copy debug info is the first entry, one pick away" },
  { look: role("menuitem", "File an issue"), say: "File an issue, which still opens its review and sends nothing" },
  { look: role("menuitem", "Reload Bridge"), say: "And Reload Bridge, for a failure a redraw can clear" },
  { press: button("Dispatch job"), say: "One press starts a Job in Armada's repository" },
  { look: role("textbox", "Message"), say: "It is open, and the toast is gone" },
  { look: inside(thread, text(/bridge\.uncaught\.rejection/)), say: "Its first message is the debug info, the text Copy debug info puts on the clipboard" },
]);

export { failureDispatchSession as "failure-dispatch-session" };
