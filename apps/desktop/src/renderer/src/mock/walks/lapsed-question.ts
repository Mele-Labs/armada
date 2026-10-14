// A question the agent asked and nobody answered in time stays under the thread as the deck, and
// answering it sends the choice to the agent as the person's own message. A waiting item that
// offers choices is the same deck in the Needs you band. Over the `lapsed-question` scenario.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";

const { rail } = kit(false);
const sessions = region("Sessions");
const thread = region("Thread");
const lapsed = role("article", "Unanswered");
const needs = region("Needs you");

const lapsedQuestion = walk("lapsed-question", [
  toSessions,
  { press: inside(sessions, button(/Book the table/)), say: "A Session whose question timed out" },
  { look: inside(thread, text("AskUserQuestion: no answer")), say: "The thread keeps a line for it" },
  { look: inside(lapsed, text("Which night?")), say: "The question is still under the thread, with its options" },
  { press: inside(lapsed, role("radio", "Friday")), say: "Friday" },
  { press: inside(lapsed, button("Answer")), say: "Answered" },
  { look: inside(thread, text(/Answering your earlier question "Which night\?": Friday/)), say: "The choice goes to the agent as the person's message, and the deck is gone" },
  { press: rail("Sessions"), say: "Back to the list" },
  { press: inside(sessions, button(/Pick the venue/)), say: "A Session whose question was skipped" },
  { look: inside(lapsed, text("Which part of town?")), say: "A skipped question comes back as the deck, so a press by mistake can be undone" },
  { press: inside(lapsed, button("Skip")), say: "Skip puts it away" },
  { press: rail("Sessions"), say: "Back to the list" },
  { press: inside(sessions, button(/Tidy the fixtures/)), say: "A Session whose waiting item has choices" },
  { press: inside(needs, button(/^Needs you/)), say: "Needs you, opened" },
  { look: inside(needs, text("Keep the old fixture or drop it?")), say: "The item is the deck, with what each choice means" },
  { press: inside(needs, role("radio", "Drop")), say: "Drop" },
  { press: inside(needs, button("Answer")), say: "Answered" },
  { look: inside(thread, text("Going with Drop.")), say: "The agent carries on from the choice" },
]);

export { lapsedQuestion };
