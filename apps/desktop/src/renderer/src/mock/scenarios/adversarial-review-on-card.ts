// Take A of where an adversarial review's findings go: the press beside Approve and Not yet on the plan card, and the findings on the card
// itself, between the plan and its answers. The walk `adversarialReviewOnCard` plays it. Over the `one-session-view` scenario (`../adversarial-review.ts`).

import { adversarialReview } from "../adversarial-review";

export const adversarialReviewOnCard = adversarialReview(
  "adversarial-review-on-card",
  "A plan waiting for approval, with an adversarial review whose three findings come onto the card between the plan and its answers",
  "card",
);
