// Every Job a Job starts is on its spine: a group handed to a Job still working stays in Now though its tasks are ticked, and a Job started outside the
// plan stands in Now while it runs and in Done once it ends. Artifacts lists them all. Over the `linked-jobs-on-plan` scenario.

import { button, deckRow, region, walk } from "../walk";

const jobsOutsideThePlan = walk("linked-jobs-on-plan", [
  { press: deckRow("Move the store to named migrations"), say: "The parent. It handed seven groups to Jobs and started two more outside its plan" },
  { look: button("Job Migrations apply in order"), say: "Every task in this group is ticked, but its Job is still working, so it stays in Now" },
  { look: button("Job Seed the test database"), say: "Started outside the plan and still running: in Now, with its pull request" },
  { press: button(/^Done/), say: "Open Done" },
  { look: button("Job Check an old dump restores"), say: "Started outside the plan and ended: in Done" },
  { press: button("Artifacts", { exact: true }), say: "Artifacts" },
  { look: region("Artifacts", { exact: true }), say: "Jobs lists both, with no group line" },
]);

export { jobsOutsideThePlan as "jobs-outside-the-plan" };
