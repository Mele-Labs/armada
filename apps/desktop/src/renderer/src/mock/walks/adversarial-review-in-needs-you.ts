// Take B: the same press on the plan card, which then shows only the review's mark, and each finding a call in the Needs you panel on the
// concern card. Over the `adversarial-review-in-needs-you` scenario.

import { button, deckRow, inside, region, role, text, walk } from "../walk";

const card = role("article", "Plan to approve");
const needs = region("Needs you");

export const adversarialReviewInNeedsYou = walk("adversarial-review-in-needs-you", [
  { press: deckRow("Warm the next bay"), say: "The Job is open, plan v1 waiting" },
  { press: inside(card, button("Adversarial review")), say: "The press is the same, in the answers row. A turning mark stands in its place while it reads" },
  { look: inside(card, role("img", "Adversarial review done")), say: "Done. The card keeps only the mark, and the findings are in Needs you" },
  { press: inside(needs, button("Needs you", { exact: true })), say: "The band opens on the findings" },
  { press: inside(needs, button(/^Open A lease waits/)), say: "A finding opens as a concern card" },
  { look: inside(needs, text("The warm skips a held slot and tries the next")), say: "What breaks, then Expected and Found, and where it cites" },
  { press: inside(needs, button("Send to job")), say: "Send goes to the agent and the finding leaves the list" },
  { press: inside(needs, button(/^Open A bay reads/)), say: "The next finding" },
  { press: inside(needs, button("Dismiss", { exact: true })), say: "Dismiss leaves it unsent" },
]);
