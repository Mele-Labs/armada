// The Answer button in a question deck: plain until every question has an answer, the primary act once they do.
// Over the `session-question` scenario.

import { toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, walk } from "../walk";

const sessions = region("Jobs", { exact: true });
const ask = role("article", "Waiting on you");

const answerButton = walk("session-question", [
  toSessions,
  { press: inside(sessions, button(/Order the lunch/)), say: "A Job with two questions waiting" },
  { press: inside(ask, role("radio", "Large")), say: "Size" },
  { press: inside(ask, role("checkbox", "Cheese")), say: "Toppings" },
  { look: inside(ask, button("Answer")), say: "Answer is the primary button once everything is answered" },
]);

export { answerButton as "session-question-answer-button" };
