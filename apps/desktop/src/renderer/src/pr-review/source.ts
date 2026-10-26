// What the review store reads and writes: the calls Fleet answers (`shared/api/reviews.ts`), each as the thing
// answered or the words of its refusal. **The store holds no `window.armada`**, so a test hands it a fake one and the
// mock Fleet answers the same calls the real one does.

import type {
  ForgetReviewSearch,
  OpenReviewWalk,
  PullRequestFilePatch,
  PullRequestFilePatches,
  PullRequestPage,
  PullReviewLists,
  PullThreadResolved,
  RememberedPlace,
  RememberedPlaces,
  ResolvePullThread,
  ReviewRun,
  ReviewRuns,
  ReviewTools,
  ReviewWalk,
  SaveReviewTool,
  SaveWalkPlace,
  StartReviewRun,
  StartReviewSearch,
  WalkPlaceFound,
  WritePullRequest,
} from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";

import type { ReviewActed, ReviewsApi } from "../../../shared/api/reviews";
import type { ReviewWalkNote, ShownReviewWalk, ShownReviewWalkOutcome } from "../../../shared/review-walk";

/** An answer, or Fleet's words for why there is none. For a forge that would not take a write, the words are the forge's own. */
export type Read<T> = { ok: true; value: T } | { ok: false; said: string };

export type ReviewSource = {
  lists: () => Promise<Read<PullReviewLists>>;
  page: (manifestId: string, number: number) => Promise<Read<PullRequestPage>>;
  file: (manifestId: string, number: number, path: string) => Promise<Read<PullRequestFilePatch>>;
  /** Every file's patch, in the forge's order, from one read: what the Files tab draws. */
  patches: (manifestId: string, number: number) => Promise<Read<PullRequestFilePatches>>;
  write: (manifestId: string, write: WritePullRequest) => Promise<Read<PullRequestPage>>;
  resolve: (manifestId: string, resolve: ResolvePullThread) => Promise<Read<PullThreadResolved>>;
  tools: (manifestId: string) => Promise<Read<ReviewTools>>;
  saveTool: (manifestId: string, save: SaveReviewTool) => Promise<Read<ReviewTools>>;
  startRun: (manifestId: string, start: StartReviewRun) => Promise<Read<ReviewRun>>;
  runs: (manifestId: string, number: number) => Promise<Read<ReviewRuns>>;
  /** A run moved, whole. Not replayed: a reconnect reads `runs` again. */
  onRun: (onRun: (run: ReviewRun) => void) => () => void;
  /** Natural Search: one run across every repository, or the one kept for these words. Its progress arrives on `onRun`. */
  startSearch: (start: StartReviewSearch) => Promise<Read<ReviewRun>>;
  /** Every search kept or going, newest first. */
  searches: () => Promise<Read<ReviewRuns>>;
  forgetSearch: (forget: ForgetReviewSearch) => Promise<Read<ReviewRuns>>;
  /** Where a pull request's change runs: from the pull request, then from memory. Reads and spends nothing. */
  findPlace: (manifestId: string, number: number) => Promise<Read<WalkPlaceFound>>;
  places: () => Promise<Read<RememberedPlaces>>;
  savePlace: (manifestId: string, save: SaveWalkPlace) => Promise<Read<RememberedPlace>>;
  forgetPlace: (manifestId: string, id: string) => Promise<Read<RememberedPlaces>>;
  openWalk: (manifestId: string, open: OpenReviewWalk) => Promise<Read<ReviewWalk>>;
  closeWalk: (manifestId: string, id: string) => Promise<Read<ReviewWalk>>;
  /** A held walk moved, whole. Not replayed. */
  onWalk: (onWalk: (walk: ReviewWalk) => void) => () => void;
  /** Opens Armada's walk window on a ready walk. */
  showWalk: (shown: ShownReviewWalk) => Promise<ShownReviewWalkOutcome>;
  /** A note taken in a walk window that no Session holds. */
  onNote: (onNote: (note: ReviewWalkNote) => void) => () => void;
  /** A walk window closed, and its walk with it. */
  onClosed: (onClosed: (walkId: string) => void) => () => void;
};

const worded = <T>(answer: ReviewActed<T>): Read<T> => (answer.ok ? { ok: true, value: answer.value } : { ok: false, said: refusalWords(answer.outcome) });

/** The source over what the preload (or the mock Fleet) offers. */
export const reviewSourceOf = (api: ReviewsApi): ReviewSource => ({
  lists: async () => worded(await api.getReviewLists()),
  page: async (manifestId, number) => worded(await api.readReviewPage(manifestId, number)),
  file: async (manifestId, number, path) => worded(await api.readReviewFile(manifestId, number, path)),
  patches: async (manifestId, number) => worded(await api.readReviewPatches(manifestId, number)),
  write: async (manifestId, write) => worded(await api.writeReview(manifestId, write)),
  resolve: async (manifestId, resolve) => worded(await api.resolveReviewThread(manifestId, resolve)),
  tools: async (manifestId) => worded(await api.listReviewTools(manifestId)),
  saveTool: async (manifestId, save) => worded(await api.saveReviewTool(manifestId, save)),
  startRun: async (manifestId, start) => worded(await api.startReviewRun(manifestId, start)),
  runs: async (manifestId, number) => worded(await api.getReviewRuns(manifestId, number)),
  onRun: (onRun) => api.onReviewRun(onRun),
  startSearch: async (start) => worded(await api.startReviewSearch(start)),
  searches: async () => worded(await api.getReviewSearches()),
  forgetSearch: async (forget) => worded(await api.forgetReviewSearch(forget)),
  findPlace: async (manifestId, number) => worded(await api.findWalkPlace(manifestId, number)),
  places: async () => worded(await api.listWalkPlaces()),
  savePlace: async (manifestId, save) => worded(await api.saveWalkPlace(manifestId, save)),
  forgetPlace: async (manifestId, id) => worded(await api.forgetWalkPlace(manifestId, id)),
  openWalk: async (manifestId, open) => worded(await api.openReviewWalk(manifestId, open)),
  closeWalk: async (manifestId, id) => worded(await api.closeReviewWalk(manifestId, id)),
  onWalk: (onWalk) => api.onReviewWalk(onWalk),
  showWalk: (shown) => api.showReviewWalk(shown),
  onNote: (onNote) => api.onReviewWalkNote(onNote),
  onClosed: (onClosed) => api.onReviewWalkClosed(onClosed),
});
