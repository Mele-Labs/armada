// A Session's question in words, as a call: pressing its pulsing row in Work opens the Session at the end of
// its thread, the question last, and the Session's own box to answer in, with its own Send. Over
// `session-call-composer`.

import { button, deckRow, inside, region, role, text, walk } from "../walk";

const session = region(/^Session: Cap the retry ceiling/);

const sessionCallComposer = walk("session-call-composer", [
  { press: deckRow(/Cap the retry ceiling/), say: "A Session's question: its row opens the Session" },
  { look: inside(session, text("Where should the cap live, and at what value?")), say: "The end of its thread, the question last" },
  { look: inside(session, button("Send")), say: "The box has its own Send" },
  { look: inside(session, role("textbox", "Message")), say: "The Session's own box, as the Session draws it" },
  { type: "On the client, 30s\n", into: inside(session, role("textbox", "Message")), say: "Enter sends the words to the Session, and the call leaves" },
]);

export { sessionCallComposer };
