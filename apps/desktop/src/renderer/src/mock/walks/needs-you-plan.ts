// A plan waiting for approval is one row in Needs you, which stays folded while the plan is alone there. Its press opens the plan beside the
// thread as the same diff the thread's card draws, with Edit for the rows he can mark, and Approve. Over the `plan-revision-diff` scenario.

import { button, deckRow, dialog, inside, region, role, text, walk } from "../walk";

const needs = region("Needs you");
const row = inside(needs, button("Open Plan to approve"));
const sheet = dialog("Plan");

const needsYouPlan = walk("plan-revision-diff", [
  { press: deckRow("Warm the next bay"), say: "A Job with a plan waiting" },
  { look: role("article", "Plan to approve"), say: "The plan is in the thread, whole, and Needs you is folded over nothing" },
  { press: inside(needs, button("Needs you")), say: "Needs you opens" },
  { look: row, say: "The plan is one row in it" },
  { press: row, say: "A press opens it beside the thread" },
  { look: inside(sheet, role("list", "Plan")), say: "The plan as the thread draws it: unchanged groups one line, the changed group open" },
  { look: inside(sheet, text("Say how long the warm took")), say: "Added, renamed with the old text struck, and the removed task struck, as in the thread" },
  { look: inside(sheet, button("Approve", { exact: true })), say: "Approve at the foot" },
  { press: inside(sheet, button("Edit plan")), say: "Edit plan switches to the rows he can mark" },
  { look: inside(sheet, button(/^Group: /)), say: "Every group open, its title and tasks taking edits where they stand" },
  { press: inside(sheet, button("Show changes")), say: "Show changes returns to the diff" },
  { look: inside(sheet, button("Approve", { exact: true })), say: "Approve is at the foot in both" },
]);

export { needsYouPlan as "needs-you-plan" };
