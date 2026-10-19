// Jumping into a Session whose agent asked something opens Needs you on it, every time, even after it was folded. Over `dashboard-needs-you`.

import { button, inside, region, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const sessions = region("Jobs", { exact: true });
const needs = region("Needs you");

const needsYouOpensOnEntry = walk("dashboard-needs-you", [
  toSessions,
  { press: inside(sessions, button(/Flaky store test/)), say: "A Job whose agent asked a question" },
  { look: inside(needs, button(/^Open /)), say: "Needs you is already open on it" },
  { press: inside(needs, button(/^Needs you/)), say: "Folded away" },
  { press: inside(sessions, button(/Migration notes/)), say: "Another Job, with nothing waiting" },
  { press: inside(sessions, button(/Flaky store test/)), say: "Back into the first" },
  { look: inside(needs, button(/^Open /)), say: "Open again, on the question" },
]);

export { needsYouOpensOnEntry as "needs-you-opens-on-entry" };
