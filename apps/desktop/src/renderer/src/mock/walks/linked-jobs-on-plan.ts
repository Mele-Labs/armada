// A plan group handed to a Job shows that Job on its row while the group is shut, and the Artifacts list opens on its Jobs. Over the `linked-jobs-on-plan` scenario.

import { button, deckRow, region, role, walk } from "../walk";

const thread = role("region", "Thread", { exact: true });

const linkedJobsOnPlan = walk("linked-jobs-on-plan", [
  { press: deckRow("Move the store to named migrations"), say: "The parent. Its plan has eight groups, all shut" },
  { look: button("Apply them in order", { exact: true }), say: "A group handed to a Job carries it on its row: the Job's state, its pull request, and a press to open it. The last group was not handed to a Job and has none" },
  { press: button("Open Migrations apply in order"), say: "One press opens the Job" },
  { key: "Escape", on: thread, say: "Esc is Back. The parent returns with its groups shut as they were" },
  { press: button("Artifacts", { exact: true }), say: "Artifacts lists the Jobs first" },
  { look: region("Artifacts", { exact: true }), say: "The pull requests and links follow" },
]);

export { linkedJobsOnPlan as "linked-jobs-on-plan" };
