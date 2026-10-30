// The Nothing left to do panel opens with a plain summary of what the Job delivered, above the eleven pull requests it made. Over the `session-all-done` scenario.

import { button, inside, region, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const panel = region("Nothing left to do");

const doneSummary = walk("session-all-done", [
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button(/Pin the store clock/)), say: "A Job that said it is done, after eleven pull requests" },
  { look: inside(panel, button("Nothing left to do")), say: "The summary stands first, then the pull requests, which scroll" },
  { look: inside(panel, button("End job")), say: "End job and Keep going stay in view" },
]);

export { doneSummary as "done-summary" };
