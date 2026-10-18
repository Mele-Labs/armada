// A Session's question in words, as a call: pressing it in Incoming calls opens the Session at the end of
// its thread, the question last, and the Session's own box to answer in, with its own Send. Over
// `session-call-composer`.

import { button, inside, region, role, text, walk } from "../walk";

const session = region(/^Job: Cap the retry ceiling/);

const sessionCallComposer = walk("session-call-composer", [
  { press: inside(region("Incoming calls"), button(/^Job: Cap the retry ceiling/)), say: "A Job's question: the call opens the Job" },
  { press: inside(session, button(/Open Where should the cap live/)), say: "The question is a row in the panel, and a press opens it" },
  { look: inside(session, text("Where should the cap live, and at what value?")), say: "The question, in the panel over the thread" },
  { look: inside(session, button("Send")), say: "The box has its own Send" },
  { look: inside(session, role("textbox", "Message")), say: "The Job's own box, as the Job draws it" },
  { type: "On the client, 30s\n", into: inside(session, role("textbox", "Message")), say: "Enter sends the words to the Job, and the call leaves" },
]);

export { sessionCallComposer };
