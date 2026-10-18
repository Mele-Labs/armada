// A Session's question in words, as a call: pressing its pulsing row in Work opens the Session at the end of
// its thread, the question last, and the Session's own box to answer in, with its own Send. Over
// `session-call-composer`.

import { button, deckRow, inside, region, role, text, walk } from "../walk";

const session = region(/^Job: Cap the retry ceiling/);
const needs = region("Needs you");

const sessionCallComposer = walk("session-call-composer", [
  { press: deckRow(/Cap the retry ceiling/), say: "A Job's question: its row opens the Job" },
  { press: inside(needs, button(/^Needs you/)), say: "The question is in its Needs you panel" },
  { press: inside(needs, button(/Open Where should the cap live/)), say: "A press on its row opens it" },
  { look: inside(needs, text("Where should the cap live, and at what value?")), say: "The question, in the panel over the thread" },
  { look: inside(needs, button("Send")), say: "The box has its own Send" },
  { look: inside(session, role("textbox", "Message")), say: "The Job's own box, as the Job draws it" },
  { type: "On the client, 30s\n", into: inside(session, role("textbox", "Message")), say: "Enter sends the words to the Job, and the call leaves" },
]);

export { sessionCallComposer };
