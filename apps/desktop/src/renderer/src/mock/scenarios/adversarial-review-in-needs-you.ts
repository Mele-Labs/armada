// Take B of where an adversarial review's findings go: the same press on the plan card, which then shows only the review's mark, and each
// finding arriving as a call in the Needs you panel on the judge's concern card. The walk `adversarialReviewInNeedsYou` plays it. Over the
// `one-session-view` scenario (`../adversarial-review.ts`).

import { adversarialReview } from "../adversarial-review";

export const adversarialReviewInNeedsYou = adversarialReview(
  "adversarial-review-in-needs-you",
  "A plan waiting for approval, with an adversarial review whose three findings arrive as calls in the Needs you panel",
  "needs-you",
);
