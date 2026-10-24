// Take A: an adversarial review of the plan, its press beside Approve and Not yet and its findings on the card between the plan and its
// answers. Over the `adversarial-review-on-card` scenario.

import { button, deckRow, inside, role, walk } from "../walk";

const card = role("article", "Plan to approve");
const first = inside(card, role("group", /^Finding: A lease waits/));
const second = inside(card, role("group", /^Finding: A bay reads/));

export const adversarialReviewOnCard = walk("adversarial-review-on-card", [
  { press: deckRow("Warm the next bay"), say: "The Job is open, plan v1 waiting" },
  { press: inside(card, button("Adversarial review")), say: "Adversarial review sits in the answers row, beside Approve and Not yet. R presses it" },
  { look: inside(card, role("img", "Adversarial review running")), say: "A turning mark while it reads" },
  { look: first, say: "Three findings arrive on the card, above the answers. Each is headed by what breaks, with Expected, Found and where it cites" },
  { press: inside(first, button("Send to agent")), say: "Send to agent folds the finding to one line" },
  { look: inside(card, role("group", /^Finding sent: A lease waits/)), say: "The mark on the line says Sent. Focus passes to the next finding" },
  { key: "d", on: second, say: "D dismisses the finding in focus. Enter sends it" },
  { look: inside(card, role("group", /^Finding dismissed: A bay reads/)), say: "Dismissed, on one line" },
]);
