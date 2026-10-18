// A plan waiting for approval is one row in Needs you, and its press opens the plan in full beside the thread, with Approve.
// Over the `plan-revision-diff` scenario.

import { button, deckRow, dialog, inside, region, walk } from "../walk";

const needs = region("Needs you");
const row = inside(needs, button("Open Plan to approve"));
const sheet = dialog("Plan");

const needsYouPlan = walk("plan-revision-diff", [
  { press: deckRow("Warm the next bay"), say: "A Job with a plan waiting" },
  { look: row, say: "The plan is one row in Needs you" },
  { press: row, say: "A press opens it beside the thread" },
  { look: inside(sheet, button("Approve", { exact: true })), say: "The whole plan, with Approve" },
]);

export { needsYouPlan as "needs-you-plan" };
