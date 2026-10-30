// The Nothing left to do panel opens with a plain summary of what the Job delivered, above what it made. Over the `session-head` scenario.

import { button, inside, region, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const head = region("Job s-c9e3a5f1");

const doneSummary = walk("session-head", [
  toSessions,
  { press: inside(region("Jobs", { exact: true }), role("button", /Release notes draft/)), say: "A Job that said it is done" },
  { look: inside(head, button("Nothing left to do")), say: "The summary stands first in the panel, above what was made" },
  { look: inside(head, button("End job")), say: "Then the two ways on" },
]);

export { doneSummary as "done-summary" };
