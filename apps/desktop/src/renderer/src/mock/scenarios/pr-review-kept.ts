// The review surface on a relaunch: the last reading of the lists is kept from 20 minutes ago, one pull request short, and
// Fleet answers four seconds after the view opens, with that one in. The walk `prListsKept` plays it.

import type { Scenario } from "../moment";
import { reviewSeed } from "../pr-review-pages";
import { s262PrReview as base } from "./pr-review";

export const s262PrReviewKept: Scenario = {
  ...base,
  name: "pr-review-kept",
  says: "The review surface on a relaunch: a reading of the lists 20 minutes old is drawn at once, and Fleet's answer follows with one more pull request",
  draft: { ...base.draft, prReview: { ...reviewSeed(), kept: { minutesAgo: 20, without: "mele-labs/pocket#212", answersAfterMs: 4000 } } },
};
