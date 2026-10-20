// Four questions in the Needs you panel: a stepper beside Back that fills as they are answered and takes him to any of them, and
// answers that are still there after leaving the Job. Over the `session-question-many` scenario.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";

const { rail } = kit(false);
const sessions = region("Jobs", { exact: true });
const ask = role("article", "Waiting on you");
const steps = inside(ask, role("group", "Questions"));

const many = walk("session-question-many", [
  toSessions,
  { press: inside(sessions, button(/Plan the offsite/)), say: "A Job with four questions waiting" },
  { look: steps, say: "One node per question, beside Back" },
  { press: inside(ask, role("radio", "Lisbon")), say: "City" },
  { press: inside(ask, role("radio", "Mar 10")), say: "Week" },
  { press: inside(ask, role("radio", "Loft")), say: "Venue" },
  { look: inside(ask, text("Which catering?")), say: "Three answered and checked, the last one ringed" },
  { hover: inside(steps, button("Week: Mar 10")), say: "Hover names what was chosen" },
  { press: inside(steps, button("Week: Mar 10")), say: "Any node takes him back to its question" },
  { press: rail("Sessions"), say: "Leaving the Job" },
  { press: inside(sessions, button(/Plan the offsite/)), say: "And coming back" },
  { look: inside(steps, button("Venue: Loft")), say: "The answers are still there, kept like a message not yet sent" },
]);

export { many as "session-question-many" };
