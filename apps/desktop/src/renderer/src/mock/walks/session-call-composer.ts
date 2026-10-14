// A Session's question in words, as a call: the end of its thread, the question, the acts in a row, and
// the Session's own box to answer in, with its own Send. Over `session-call-composer`.

import { button, inside, region, role, walk } from "../walk";

const card = region(/^Session: Cap the retry ceiling/);

const sessionCallComposer = walk("session-call-composer", [
  { press: inside(role("group", "Calls waiting"), button(/^Session: Cap the retry ceiling/)), say: "The Session's call waits in the tray among the Jobs', and a press opens it" },
  { look: card, say: "A Session's question, with the end of its thread" },
  { look: button(/Open Session/), say: "The acts are on the rail beside the card, the box under the thread" },
  { look: inside(card, role("textbox", "Message")), say: "The Session's own box, as the Session draws it" },
  { type: "On the client, 30s\n", into: inside(card, role("textbox", "Message")), say: "Enter sends the words to the Session, and the call leaves" },
]);

export { sessionCallComposer };
