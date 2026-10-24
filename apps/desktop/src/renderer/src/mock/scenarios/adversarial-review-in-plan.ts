// Take C of where an adversarial review's findings go: pinned to the plan itself. A group holding findings has a mark at the end of its line
// and opens in place to the task each names, with the finding directly under it; a finding about a group sits under its line and one about
// the whole plan under the summary. The full plan view draws the same. The walk `adversarialReviewInPlan` plays it. Over the
// `one-session-view` scenario (`../adversarial-review.ts`).

import { adversarialReview } from "../adversarial-review";

export const adversarialReviewInPlan = adversarialReview(
  "adversarial-review-in-plan",
  "A plan waiting for approval, with an adversarial review whose four findings are pinned to the plan: one to the whole of it, one to a group, two to tasks",
  "in-plan",
);
