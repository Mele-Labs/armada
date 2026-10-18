// A question the agent asked and nobody answered in time stays as the deck in the Needs you panel on the composer, and
// answering it sends the choice to the agent as the person's own message. A waiting item that
// offers choices is the same deck in the panel. Over the `lapsed-question` scenario.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";

const { rail } = kit(false);
const sessions = region("Jobs", { exact: true });
const thread = region("Thread");
const lapsed = role("article", "Unanswered");
const needs = region("Needs you");

const lapsedQuestion = walk("lapsed-question", [
  toSessions,
  { press: inside(sessions, button(/Book the table/)), say: "A Job whose question timed out" },
  { look: inside(thread, text("AskUserQuestion: no answer")), say: "The thread keeps a line for it" },
  { look: inside(lapsed, text("Which night?")), say: "The panel opens on it, the question still there with its options" },
  { press: inside(lapsed, role("radio", "Friday")), say: "Friday" },
  { press: inside(lapsed, button("Answer")), say: "Answered" },
  { look: inside(thread, text(/Answering your earlier question "Which night\?": Friday/)), say: "The choice goes to the agent as the person's message, and the deck is gone" },
  { press: rail("Sessions"), say: "Back to the list" },
  { press: inside(sessions, button(/Pick the venue/)), say: "A Job whose question was skipped" },
  { look: inside(lapsed, text("Which part of town?")), say: "A skipped question comes back as the deck in the panel, so a press by mistake can be undone" },
  { press: inside(lapsed, button("Skip")), say: "Skip puts it away" },
  { press: rail("Sessions"), say: "Back to the list" },
  { press: inside(sessions, button(/Tidy the fixtures/)), say: "A Job whose waiting item has choices" },
  { press: inside(needs, button(/^Needs you/)), say: "Needs you opens on the one item" },
  { look: inside(needs, text("Keep the old fixture or drop it?")), say: "The item is the deck, with what each choice means" },
  { press: inside(needs, role("radio", "Drop")), say: "Drop" },
  { press: inside(needs, button("Answer")), say: "Answered" },
  { look: inside(thread, text("Going with Drop.")), say: "The agent carries on from the choice" },
]);

export { lapsedQuestion };
