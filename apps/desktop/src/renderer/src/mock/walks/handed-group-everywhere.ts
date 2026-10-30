// A plan group handed to a Job is that Job's card in Open and in Done, as it is on the spine, and a finished one has no ticks. Over the `linked-jobs-on-plan` scenario.

import { button, deckRow, role, walk } from "../walk";

const thread = role("region", "Thread", { exact: true });

const handedGroupEverywhere = walk("linked-jobs-on-plan", [
  { press: deckRow("Move the store to named migrations"), say: "The parent. Its plan has eight groups" },
  { look: button("Job Migrations get names"), say: "In Open a handed group is the Job's card: the Job's title, the group's title under it. No second row under it" },
  { press: button(/^Done/), say: "A press on the Done band opens it" },
  { look: button("Job Roll one migration back"), say: "A finished handed group is the same card, with its pull request under it. No ticks" },
  { press: button("Job Roll one migration back"), say: "One press opens the Job" },
  { key: "Escape", on: thread, say: "Esc is Back. The parent returns as it was" },
]);

export { handedGroupEverywhere as "handed-group-everywhere" };
