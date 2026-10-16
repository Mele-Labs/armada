// Reviews: the pull requests waiting on a person and the acts of a review on one, read and written through
// Fleet. A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { Outcome, PullRequestFilePatch, PullRequestPage, PullReviewLists, PullThreadResolved, ResolvePullThread, WritePullRequest } from "@armada/protocol";

/** What a review act came to: the thing Fleet answered with, or the refusal to word. The forge's own sentence rides in the outcome. */
export type ReviewActed<T> = { ok: true; value: T } | { ok: false; outcome: Outcome };

export type ReviewsApi = {
  /** The four lists, read from the forge now. **Called when the tab opens and on Refresh, never on a timer.** */
  getReviewLists: () => Promise<ReviewActed<PullReviewLists>>;
  /** One pull request's page, any open one of a repository Fleet serves, named by the repository's manifest id. */
  readReviewPage: (manifestId: string, number: number) => Promise<ReviewActed<PullRequestPage>>;
  /** One file of it as a patch, read when the file is opened. */
  readReviewFile: (manifestId: string, number: number, path: string) => Promise<ReviewActed<PullRequestFilePatch>>;
  /** A review: the verdict, its summary and its line comments, posted as one. Answers the page read again. */
  writeReview: (manifestId: string, write: WritePullRequest) => Promise<ReviewActed<PullRequestPage>>;
  /** Resolves or opens again one conversation on the diff. */
  resolveReviewThread: (manifestId: string, resolve: ResolvePullThread) => Promise<ReviewActed<PullThreadResolved>>;
};

export const REVIEWS_CHANNELS = {
  getReviewLists: "bridge:get-review-lists",
  readReviewPage: "bridge:read-review-page",
  readReviewFile: "bridge:read-review-file",
  writeReview: "bridge:write-review",
  resolveReviewThread: "bridge:resolve-review-thread",
} as const;
