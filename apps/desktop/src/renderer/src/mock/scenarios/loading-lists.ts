// The lists that read from a forge or a tracker, before either has answered: Issues with its trackers read and every search held
// pending, Reviews with the forge's lists held pending. Each draws the shape of its rows and nothing arrives. The pages are the
// ones `issues-tab` and `pr-review` land on.

import { reviewSeed } from "../pr-review-pages";
import type { Scenario } from "../moment";
import { s321IssuesTab } from "./issues-tab";

function build(): Scenario {
  const base = s321IssuesTab;
  return {
    ...base,
    name: "loading/lists",
    says: "Issues and Reviews while the forge and the trackers have not answered: skeleton rows, nothing landing",
    draft: {
      ...base.draft,
      issuesTab: (base.draft?.issuesTab ?? []).map((one) => ({
        ...one,
        unanswered: "run" as const,
      })),
      prReview: reviewSeed(),
    },
    behaves: (...args) => ({
      ...base.behaves?.(...args),
      getReviewLists: () => new Promise(() => {}),
    }),
  };
}

export const loadingLists: Scenario = build();
