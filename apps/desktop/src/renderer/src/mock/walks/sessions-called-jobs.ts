// What was a Session is a Job to the person reading: the Cockpit's left panel, the open Job's frame and head,
// the Dispatch sheet's kinds, a call on its Job's row and a row's hover card. Ids, Helm and code are as they were.
// Over `cockpit-ship-flightdeck`, for the owner to approve before the tests follow.

import { inside, region, role, tab, walk } from "../walk";

const jobs = region("Jobs", { exact: true });
const dispatch = role("dialog", "Dispatch");

const sessionsCalledJobs = walk("cockpit-ship-flightdeck", [
  { look: jobs, say: "The left panel is headed Jobs" },
  { press: inside(jobs, role("button", "Migration notes")), say: "Pick a Job from the panel" },
  { look: region(/^Job: Migration notes/), say: "Its frame and head say Job" },
  { key: "Escape", on: region(/^Job: Migration notes/), say: "Esc closes the Job" },
  { hover: inside(inside(jobs, role("button", /^Cache key/)), role("img", "Waiting on you")), say: "The state glyph of a row names the Job on hover" },
  { hover: inside(jobs, role("button", /^Cache key/)), say: "A Job calling: its row pulses, and its tooltip names the call" },
  { press: inside(jobs, role("button", /^Cache key/)), say: "Pressing the row opens the Job" },
  { look: region(/^Job: Cache key/), say: "The Job, its question in the thread" },
  { key: "n", on: jobs, say: "n opens the Dispatch sheet" },
  { look: inside(dispatch, tab("Job")), say: "Job" },
  { look: inside(dispatch, tab("Autopilot")), say: "Autopilot" },
]);

export { sessionsCalledJobs };
