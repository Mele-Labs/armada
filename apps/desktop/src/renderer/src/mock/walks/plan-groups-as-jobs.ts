// A plan group that goes to a Job of its own draws as that Job before it starts: a dashed card with its issue and tasks inside. Once handed it is the Job's own card, and
// each issue is drawn once, under its group or in Open. Over `plan-groups-as-jobs`.

import { button, deckRow, inside, region, role, text, walk } from "../walk";

const plan = region("Plan", { exact: true });
const dispatch = region("Dispatch", { exact: true });
const open = region("Open", { exact: true });
const first = inside(plan, role("group", /^Start the Job on the workflow a rule names, not started yet$/));

const planGroupsAsJobs = walk("plan-groups-as-jobs", [
  { press: deckRow("Fix four filed issues"), say: "A Job at its Plan step, with a plan of four groups waiting for approval" },
  { look: first, say: "A group that goes to a Job of its own is that Job, not started. Its issue and tasks stay inside" },
  { hover: inside(first, role("img", "Job, not started yet")), say: "Not started yet" },
  { look: inside(open, text("armada-m8r")), say: "Open holds the one issue no group names. The other four stand in their groups" },
  { later: plan, say: "The plan is approved and the first two groups are handed to Jobs" },
  { look: inside(dispatch, button("Job A start_job rule act reads the workflow it names")), say: "The first two groups are Jobs. The other two are not started" },
  { look: open, say: "Open holds the issues of the two handed groups, and the one no group names" },
]);

export { planGroupsAsJobs as "plan-groups-as-jobs" };
