// The failure toast's Dispatch session, the face of a split control with Copy debug info, File an issue and Reload Bridge behind its
// caret. A crash in the Sessions store raises the toast; one press starts a Session in Armada's repository whose first message is the
// debug info, opens it, and puts the toast away. Over `failure-dispatch-session`.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";

const { thread } = kit(false);
const sessions = region("Sessions");

const failureDispatchSession = walk("failure-dispatch-session", [
  toSessions,
  { press: inside(sessions, button("New Session")), say: "A new Session, and the Sessions store throws" },
  { look: button("Dispatch session"), say: "The failure stands over the page. The face of its one control is Dispatch session, with Dismiss beside it" },
  { press: button("More ways to act on this failure"), say: "The caret opens the rest" },
  { look: role("menuitem", "Copy debug info"), say: "Copy debug info is the first entry, one pick away" },
  { look: role("menuitem", "File an issue"), say: "File an issue, which still opens its review and sends nothing" },
  { look: role("menuitem", "Reload Bridge"), say: "And Reload Bridge, for a failure a redraw can clear" },
  { press: button("Dispatch session"), say: "One press starts a Session in Armada's repository" },
  { look: role("textbox", "Message"), say: "It is open, and the toast is gone" },
  { look: inside(thread, text(/bridge\.uncaught\.rejection/)), say: "Its first message is the debug info, the text Copy debug info puts on the clipboard" },
]);

export { failureDispatchSession as "failure-dispatch-session" };
