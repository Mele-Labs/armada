// What the review store reads and writes: the calls Fleet answers (`shared/api/reviews.ts`), each as the thing
// answered or the words of its refusal. **The store holds no `window.armada`**, so a test hands it a fake one and the
// mock Fleet answers the same calls the real one does.

import type { PullRequestFilePatch, PullRequestPage, PullReviewLists, PullThreadResolved, ResolvePullThread, ReviewRun, ReviewRuns, ReviewTools, SaveReviewTool, StartReviewRun, WritePullRequest } from "@armada/protocol";
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
  tools: (manifestId: string) => Promise<Read<ReviewTools>>;
  saveTool: (manifestId: string, save: SaveReviewTool) => Promise<Read<ReviewTools>>;
  startRun: (manifestId: string, start: StartReviewRun) => Promise<Read<ReviewRun>>;
  runs: (manifestId: string, number: number) => Promise<Read<ReviewRuns>>;
  /** A run moved, whole. Not replayed: a reconnect reads `runs` again. */
  onRun: (onRun: (run: ReviewRun) => void) => () => void;
};

const worded = <T>(answer: ReviewActed<T>): Read<T> => (answer.ok ? { ok: true, value: answer.value } : { ok: false, said: refusalWords(answer.outcome) });

/** The source over what the preload (or the mock Fleet) offers. */
export const reviewSourceOf = (api: ReviewsApi): ReviewSource => ({
  lists: async () => worded(await api.getReviewLists()),
  page: async (manifestId, number) => worded(await api.readReviewPage(manifestId, number)),
  file: async (manifestId, number, path) => worded(await api.readReviewFile(manifestId, number, path)),
  write: async (manifestId, write) => worded(await api.writeReview(manifestId, write)),
  resolve: async (manifestId, resolve) => worded(await api.resolveReviewThread(manifestId, resolve)),
  tools: async (manifestId) => worded(await api.listReviewTools(manifestId)),
  saveTool: async (manifestId, save) => worded(await api.saveReviewTool(manifestId, save)),
  startRun: async (manifestId, start) => worded(await api.startReviewRun(manifestId, start)),
  runs: async (manifestId, number) => worded(await api.getReviewRuns(manifestId, number)),
  onRun: (onRun) => api.onReviewRun(onRun),
});
