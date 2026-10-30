// A plan group that goes to a Job of its own has the Job card as a placeholder, in the rail and heading its group on the plan card in the thread. Handed, the Job's own card
// takes its place. An issue is drawn once: under its group while the plan is drafted, in Open after. Over `plan-groups-as-jobs`.

import { button, deckRow, inside, region, role, text, walk } from "../walk";

const plan = region("Plan", { exact: true });
const dispatch = region("Dispatch", { exact: true });
const open = region("Open", { exact: true });
const planCard = role("article", "Plan to approve");
const notStarted = /^Start the Job on the workflow a rule names, not started yet$/;
const first = inside(plan, role("group", notStarted));

const planGroupsAsJobs = walk("plan-groups-as-jobs", [
  { press: deckRow("Fix four filed issues"), say: "A Job at its Plan step, with a plan of four groups waiting for approval" },
  { look: first, say: "A group that goes to a Job of its own has the Job card as a placeholder where its title was. Its issue, tasks and chevron stand under it" },
  { hover: inside(first, role("img", "Job, not started yet")), say: "Not started yet" },
  { look: inside(planCard, role("group", notStarted)), say: "The plan card heads the group with the same placeholder. Its task marks and subagent type stand under it" },
  { look: inside(open, text("armada-m8r")), say: "Open holds the one issue no group names. The other four stand under their groups" },
  { later: plan, say: "The plan is approved and the first two groups are handed to Jobs" },
  { look: inside(dispatch, button("Job A start_job rule act reads the workflow it names")), say: "The first two groups are Job cards on their own. The other two are placeholder cards" },
  { look: open, say: "Open holds the issues of all four groups and the one no group names" },
]);

export { planGroupsAsJobs as "plan-groups-as-jobs" };
