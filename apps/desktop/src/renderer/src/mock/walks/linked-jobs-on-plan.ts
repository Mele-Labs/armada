// A plan group handed to a Job shows that Job on its row while the group is shut, and the Artifacts list opens on its Jobs. Over the `linked-jobs-on-plan` scenario.

import { button, deckRow, region, role, walk } from "../walk";

const thread = role("region", "Thread", { exact: true });

const linkedJobsOnPlan = walk("linked-jobs-on-plan", [
  { press: deckRow("Move the store to named migrations"), say: "The parent. Its plan has eight groups" },
  { look: button("Job Apply them in a fixed order"), say: "A group handed to a Job is the Job's row: its title, with the group's title under it. The last group was not handed to a Job and keeps its own row" },
  { press: button("Job Migrations apply in order"), say: "One press opens the Job" },
  { key: "Escape", on: thread, say: "Esc is Back. The parent returns as it was" },
  { press: button("Artifacts", { exact: true }), say: "Artifacts lists the Jobs first" },
  { look: region("Artifacts", { exact: true }), say: "The pull requests and links follow" },
]);

export { linkedJobsOnPlan as "linked-jobs-on-plan" };
