// Triggers in a Job (7 Oct 2026): one in each state a firing can be in. Each is a mark with a
// tooltip, the level that sets it beside it, and its name goes to its line in the Job's log.

import { button, dialog, inside, region, role, text, walk } from "../walk";

const LOG = dialog("Job log");

export const triggersThatFired = walk("real/job-2-at-review", [
  { look: region("Triggers"), say: "The Triggers frozen onto the Job, each with where it is set" },
  { hover: role("img", "Passed"), say: "One passed" },
  { hover: role("img", "Failed, exit 1"), say: "One failed, with its exit code" },
  { hover: role("img", /not a Command this repository declares/), say: "One skipped, with the reason" },
  { hover: role("img", "Waiting on you"), say: "One waits on the owner: it is on a destructive Command" },
  { hover: role("img", "Running"), say: "One is running" },
  { hover: role("img", "Pending"), say: "One has not been reached" },
  { press: button("gate"), say: "A firing's name goes to its line in the log" },
  { look: inside(LOG, text(/Trigger `gate` failed/)), say: "The line it wrote, open" },
]);
