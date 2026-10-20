// Four questions in the Needs you panel, answered one after another: what the answered ones look like beside Back as they pile up.
// Over the `session-question-many` scenario.

import { toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";

const sessions = region("Jobs", { exact: true });
const ask = role("article", "Waiting on you");

const many = walk("session-question-many", [
  toSessions,
  { press: inside(sessions, button(/Plan the offsite/)), say: "A Job with four questions waiting" },
  { press: inside(ask, role("radio", "Lisbon")), say: "City" },
  { press: inside(ask, role("radio", "Mar 10")), say: "Week" },
  { press: inside(ask, role("radio", "Loft")), say: "Venue" },
  { look: inside(ask, text("Which catering?")), say: "Three answered, the last in front" },
  { hover: button("Week: Mar 10"), say: "Each answer is a numbered circle beside Back; hover names it" },
  { press: button("Week: Mar 10"), say: "A press brings that question back" },
]);

export { many as "session-question-many" };
