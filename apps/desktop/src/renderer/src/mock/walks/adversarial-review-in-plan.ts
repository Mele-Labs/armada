// The review's findings pinned to the plan, under the group or task each is about, on the card and in the plan in full.
// Over the `adversarial-review-in-plan` scenario.

import { button, deckRow, dialog, inside, role, text, walk } from "../walk";

const card = role("article", "Plan to approve");
const sheet = dialog("Plan");
const lease = inside(card, role("group", /^Finding: A lease waits/));
const restart = inside(card, role("group", /^Finding: A bay reads/));

export const adversarialReviewInPlan = walk("adversarial-review-in-plan", [
  { press: deckRow("Warm the next bay"), say: "The Job is open, plan v1 waiting" },
  { press: inside(card, button("Adversarial review")), say: "A turning mark stands in its place while the adversary reads" },
  { look: inside(card, button(/^Findings on/)), say: "A group that holds findings has a mark at the end of its line, and opens under it" },
  { look: lease, say: "A finding sits under the task it is about, named by its words, so it does not repeat them. The finding about the whole plan is under the summary" },
  { press: inside(lease, button("Send to agent")), say: "Send to agent folds the finding to one line, in place" },
  { press: inside(restart, button("Dismiss", { exact: true })), say: "Dismiss does the same, unsent. J and K or the arrows move between findings, Enter sends, D dismisses" },
  { press: inside(card, text(/^The next free bay is warmed/)), say: "A press on the card opens the plan in full" },
  { look: inside(sheet, role("group", /^Finding: A warm that never finishes/)), say: "The same findings, under the same parts of the plan, with the same acts" },
]);
