// A Session that split its plan into linked Sessions it follows: each group handed off carries its Session's row in Now, and the same Sessions stand in the parent's Artifacts under Sessions.
// Over the `linked-sessions` scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";

const plan = region("Now", { exact: true });
const artifacts = region("Artifacts", { exact: true });
const thread = role("region", "Thread", { exact: true });

export const linkedSessions = walk("linked-sessions", [
  { press: deckRow("Move the store to named migrations"), say: "The parent. In Jobs its linked Jobs stand under it. In Now, opened to the whole plan, each group handed off carries the row of its Job" },
  { press: inside(plan, button("Whole plan")), say: "One press opens the whole plan" },
  { press: button("Artifacts", { exact: true }), say: "The same Jobs are in its Artifacts under Jobs: the title, then the group each came from, its state and its pull request" },
  { press: inside(artifacts, button("Back")), say: "Back returns to the thread" },
  { later: thread, say: "One of them asks a question" },
  { later: thread, say: "Another merges its pull request" },
  { look: inside(thread, button(/^One row for each migration, Merged/)), say: "The merge is in the parent's thread where it happened, and its agent is told. The question is on the asking Job's own row only" },
  { press: inside(plan, button("List them in System")), say: "A group that was not handed off opens to Hand to a Job" },
  { key: "Enter", on: inside(plan, button("Open Migrations apply in order")), say: "Enter on a Job's row opens it" },
  { key: "Escape", on: region("Job: Migrations apply in order"), say: "Esc is Back. The parent returns with its thread and its open group as they were" },
]);
