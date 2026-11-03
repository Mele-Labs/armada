// The review surface, with a commit pushed to the pull request he reviews in panes: the next read of the lists brings it, and what he ran
// on it is then on an older commit. The walk `pr-review-panes` plays it.

import type { Scenario } from "../moment";
import { reviewSeed } from "../pr-review-pages";
import { s262PrReview as base } from "./pr-review";

const seed = reviewSeed();

export const s262PrReviewPanes: Scenario = {
  ...base,
  name: "pr-review-panes",
  says: "The review surface where a pull request he ran tools on gains a commit when the lists are read again",
  draft: {
    ...base.draft,
    prReview: {
      ...seed,
      pushes: {
        ...seed.pushes,
        "mele-labs/armada#1899": {
          sha: "e4f5a6b",
          headline: "Keep the cache for the window it was read in",
          author: "marta",
        },
      },
    },
  },
};
