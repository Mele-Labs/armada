// A question answered in the Needs you panel leaves a numbered circle beside Back, naming what was chosen, and a press on it brings the
// question back. Over the `session-question` scenario.

import { toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";

const sessions = region("Jobs", { exact: true });
const ask = role("article", "Waiting on you");

const answered = walk("session-question", [
  toSessions,
  { press: inside(sessions, button(/Order the lunch/)), say: "A Job with two questions waiting" },
  { press: inside(ask, role("radio", "Large")), say: "Large, and the next comes forward" },
  { hover: button("Size: Large"), say: "The answer is a numbered circle beside Back; hover names it" },
  { press: button("Size: Large"), say: "A press brings the question back" },
  { look: inside(ask, text("Which size?")), say: "Back on the first question" },
]);

export { answered as "session-question-answered" };
