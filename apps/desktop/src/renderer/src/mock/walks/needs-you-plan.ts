// A plan waiting for approval is answered in Needs you: the panel opens on it, Approve is there, and a press on the card opens it in
// full. Over the `plan-revision-diff` scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";

const needs = region("Needs you");
const card = inside(needs, role("article", "Plan to approve"));

const needsYouPlan = walk("plan-revision-diff", [
  { press: deckRow("Warm the next bay"), say: "A Job with a plan waiting" },
  { look: card, say: "Needs you opens on the plan" },
  { look: inside(card, button("Approve", { exact: true })), say: "Approve is here" },
  { press: card, say: "A press on the card opens it in full" },
]);

export { needsYouPlan as "needs-you-plan" };
