// What the review store reads and writes: the five calls Fleet answers (`shared/api/reviews.ts`), each as the thing
// answered or the words of its refusal. **The store holds no `window.armada`**, so a test hands it a fake one and the
// mock Fleet answers the same calls the real one does.

import type { PullRequestFilePatch, PullRequestPage, PullReviewLists, PullThreadResolved, ResolvePullThread, WritePullRequest } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";

import type { ReviewActed, ReviewsApi } from "../../../shared/api/reviews";

/** An answer, or Fleet's words for why there is none. For a forge that would not take a write, the words are the forge's own. */
export type Read<T> = { ok: true; value: T } | { ok: false; said: string };

export type ReviewSource = {
  lists: () => Promise<Read<PullReviewLists>>;
  page: (manifestId: string, number: number) => Promise<Read<PullRequestPage>>;
  file: (manifestId: string, number: number, path: string) => Promise<Read<PullRequestFilePatch>>;
  write: (manifestId: string, write: WritePullRequest) => Promise<Read<PullRequestPage>>;
  resolve: (manifestId: string, resolve: ResolvePullThread) => Promise<Read<PullThreadResolved>>;
};

const worded = <T>(answer: ReviewActed<T>): Read<T> => (answer.ok ? { ok: true, value: answer.value } : { ok: false, said: refusalWords(answer.outcome) });

/** The source over what the preload (or the mock Fleet) offers. */
export const reviewSourceOf = (api: ReviewsApi): ReviewSource => ({
  lists: async () => worded(await api.getReviewLists()),
  page: async (manifestId, number) => worded(await api.readReviewPage(manifestId, number)),
  file: async (manifestId, number, path) => worded(await api.readReviewFile(manifestId, number, path)),
  write: async (manifestId, write) => worded(await api.writeReview(manifestId, write)),
  resolve: async (manifestId, resolve) => worded(await api.resolveReviewThread(manifestId, resolve)),
});
