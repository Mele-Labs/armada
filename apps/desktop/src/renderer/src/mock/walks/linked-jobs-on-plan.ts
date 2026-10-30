// A plan group handed to a Job shows that Job on its row while the group is shut, and the Artifacts list opens on its Jobs. Over the `linked-jobs-on-plan` scenario.
// `&take=card` draws the Job as the shared Job card, `&take=chip` as the Job chip with the pull request chip beside it.

import { button, deckRow, region, role, walk } from "../walk";

const thread = role("region", "Thread", { exact: true });

const linkedJobsOnPlan = walk("linked-jobs-on-plan", [
  { press: deckRow("Move the store to named migrations"), say: "The parent. Its plan has eight groups, all shut" },
  { look: button("Apply them in order", { exact: true }), say: "A group handed to a Job shows it under the group's title, with its pull request where it has one. The last group was not handed to a Job and has none" },
  { press: button(/^(Open|Job) Migrations apply in order$/), say: "One press opens the Job" },
  { key: "Escape", on: thread, say: "Esc is Back. The parent returns with its groups shut as they were" },
  { press: button("Artifacts", { exact: true }), say: "Artifacts lists the Jobs first" },
  { look: region("Artifacts", { exact: true }), say: "The pull requests and links follow" },
]);

export { linkedJobsOnPlan as "linked-jobs-on-plan" };
