// The failure toast's Dispatch session. A crash in the Sessions store raises the toast; one press starts a Session in
// Armada's repository whose first message is the debug info, opens it, and puts the toast away. Over `failure-dispatch-session`.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";

const { thread } = kit(false);
const sessions = region("Sessions");

const failureDispatchSession = walk("failure-dispatch-session", [
  toSessions,
  { press: inside(sessions, button("New Session")), say: "A new Session, and the Sessions store throws" },
  { look: button("Copy debug info"), say: "The failure stands over the page, with Copy debug info as ever" },
  { look: button("File an issue"), say: "File an issue beside it, unchanged" },
  { look: button("Dispatch session"), say: "And Dispatch session, a ghost control like its neighbours, with no glyph" },
  { look: button("Dismiss"), say: "Dismiss puts the toast away" },
  { press: button("Dispatch session"), say: "One press starts a Session in Armada's repository" },
  { look: role("textbox", "Message"), say: "It is open, and the toast is gone" },
  { look: inside(thread, text(/bridge\.uncaught\.rejection/)), say: "Its first message is the debug info, the text Copy debug info puts on the clipboard" },
]);

export { failureDispatchSession as "failure-dispatch-session" };
