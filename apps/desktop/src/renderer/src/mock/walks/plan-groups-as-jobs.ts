// A plan group that goes to a Job of its own is the Job card alone, as a placeholder, in the rail and on the plan card in the thread, its issue a chip in the card. Handed,
// the Job's own card takes its place with the same chip. An issue a Job card carries is not in Open. Over `plan-groups-as-jobs`.

import { button, deckRow, inside, region, role, text, walk } from "../walk";

const plan = region("Plan", { exact: true });
const dispatch = region("Dispatch", { exact: true });
const open = region("Open", { exact: true });
const planCard = role("article", "Plan to approve");
const notStarted = /^Start the Job on the workflow a rule names, not started yet$/;
const first = inside(plan, role("group", notStarted));

const planGroupsAsJobs = walk("plan-groups-as-jobs", [
  { press: deckRow("Fix four filed issues"), say: "A Job at its Plan step, with a plan of four groups waiting for approval" },
  { look: first, say: "A group that goes to a Job of its own is the Job card, not started yet. The Job plans its own tasks, so the group's are not drawn" },
  { hover: inside(first, role("button", /^Issue armada-95v/)), say: "Its issue is a chip in the card. A press opens it" },
  { look: inside(planCard, role("group", notStarted)), say: "The plan card in the thread shows the same card" },
  { look: inside(open, text("armada-m8r")), say: "Open holds the one issue no Job card names" },
  { later: plan, say: "The plan is approved and the first two groups are handed to Jobs" },
  { look: inside(dispatch, button("Job A start_job rule act reads the workflow it names")), say: "The first two are the Jobs' own cards, each with its issue chip. The other two are still placeholders" },
  { look: open, say: "Open still holds only the issue no Job card names" },
]);

export { planGroupsAsJobs as "plan-groups-as-jobs" };
