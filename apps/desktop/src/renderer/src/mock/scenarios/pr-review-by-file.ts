// The review surface with one more pull request asked of him, written for the Review tab: eight changed files, Risks findings on
// lines of two of them, one on a line the diff does not hold, and findings naming other changed files
// (`../pr-review-by-file-pages.ts`). fernhill/tally is one of Fleet's repositories too, so System, Repository draws its Run on
// opening. The walk `pr-review-by-file` plays it.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import type { Scenario } from "../moment";
import { refunds } from "../pr-review-by-file-pages";
import { reviewSeed } from "../pr-review-pages";
import { s262PrReview as base } from "./pr-review";

const seed = reviewSeed();

const tally = (): RepositorySummary => {
  const one = repository();
  return {
    root: "/Users/user/tally",
    records_root: "/Users/user/Library/Application Support/Armada/records/tally",
    manifest: {
      ...one.manifest!,
      id: refunds.pr.manifestId,
      repository: refunds.pr.repository,
      path: "tally/armada.yml",
      records_root: "/Users/user/Library/Application Support/Armada/records/tally",
    },
  };
};

export const sPrReviewByFile: Scenario = {
  ...base,
  name: "pr-review-by-file",
  says: "The review surface with a pull request of eight files whose Risks findings sit on lines of two of them",
  state: {
    ...base.state,
    holds: {
      ...base.state.holds,
      repositories: [
        ...(base.state.holds.repositories ?? []),
        tally(),
      ],
    },
  },
  draft: {
    ...base.draft,
    prReview: {
      ...seed,
      prs: [
        refunds.pr,
        ...seed.prs,
      ],
      results: {
        ...seed.results,
        [refunds.pr.key]: refunds.results,
      },
    },
  },
};
