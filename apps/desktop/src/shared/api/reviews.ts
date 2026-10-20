// Reviews: the pull requests waiting on a person and the acts of a review on one, read and written through
// Fleet. A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type {
  ForgetReviewSearch,
  OpenReviewWalk,
  Outcome,
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

import type { ReviewWalkNote, ShownReviewWalk, ShownReviewWalkOutcome } from "../review-walk";

/** What a review act came to: the thing Fleet answered with, or the refusal to word. The forge's own sentence rides in the outcome. */
export type ReviewActed<T> = { ok: true; value: T } | { ok: false; outcome: Outcome };

export type ReviewsApi = {
  /** The four lists, read from the forge now. **Called when the tab opens and on Refresh, never on a timer.** */
  getReviewLists: () => Promise<ReviewActed<PullReviewLists>>;
  /** One pull request's page, any open one of a repository Fleet serves, named by the repository's manifest id. */
  readReviewPage: (manifestId: string, number: number) => Promise<ReviewActed<PullRequestPage>>;
  /** One file of it as a patch, read when the file is opened. */
  readReviewFile: (manifestId: string, number: number, path: string) => Promise<ReviewActed<PullRequestFilePatch>>;
  /** Every file of it as a patch, in the forge's order, from one read. */
  readReviewPatches: (manifestId: string, number: number) => Promise<ReviewActed<PullRequestFilePatches>>;
  /** A review: the verdict, its summary and its line comments, posted as one. Answers the page read again. */
  writeReview: (manifestId: string, write: WritePullRequest) => Promise<ReviewActed<PullRequestPage>>;
  /** Resolves or opens again one conversation on the diff. */
  resolveReviewThread: (manifestId: string, resolve: ResolvePullThread) => Promise<ReviewActed<PullThreadResolved>>;
  /** The review tools and explanations a repository has, resolved Kit's over the repository's, which is over Armada's, and the files left out. */
  listReviewTools: (manifestId: string) => Promise<ReviewActed<ReviewTools>>;
  /** Writes one tool or explanation in Kit's folder or the repository's own. Answers the tools read again. */
  saveReviewTool: (manifestId: string, save: SaveReviewTool) => Promise<ReviewActed<ReviewTools>>;
  /** Starts an agent run on a pull request, or answers the one kept. The run's progress and result arrive on `onReviewRun`. */
  startReviewRun: (manifestId: string, start: StartReviewRun) => Promise<ReviewActed<ReviewRun>>;
  /** Every run kept or going for one pull request, newest first. Read when it is opened and after a reconnect. */
  getReviewRuns: (manifestId: string, number: number) => Promise<ReviewActed<ReviewRuns>>;
  /** `review_run.changed`: a run moved, whole. Not replayed, so a reconnect reads `getReviewRuns` again. */
  onReviewRun: (onRun: (run: ReviewRun) => void) => () => void;
  /** Natural Search: an agent run across the open pull requests of every served repository, or the one kept for these words. Its progress and result arrive on `onReviewRun`, as a run of kind `search`. */
  startReviewSearch: (start: StartReviewSearch) => Promise<ReviewActed<ReviewRun>>;
  /** Every search kept or going, newest first. Read when the tab opens and after a reconnect. */
  getReviewSearches: () => Promise<ReviewActed<ReviewRuns>>;
  /** Drops a kept search from Fleet, so it is gone after a restart. Answers the searches kept now. */
  forgetReviewSearch: (forget: ForgetReviewSearch) => Promise<ReviewActed<ReviewRuns>>;
  /** Where this pull request's change runs, from the pull request and then from what Fleet remembers. Reads, spends nothing. */
  findWalkPlace: (manifestId: string, number: number) => Promise<ReviewActed<WalkPlaceFound>>;
  /** Every place Fleet remembers, across the served repositories. */
  listWalkPlaces: () => Promise<ReviewActed<RememberedPlaces>>;
  /** Remembers where changes to an area of a repository run, replacing the place kept for the area. */
  saveWalkPlace: (manifestId: string, save: SaveWalkPlace) => Promise<ReviewActed<RememberedPlace>>;
  /** Stops remembering one. Answers what is kept now. */
  forgetWalkPlace: (manifestId: string, id: string) => Promise<ReviewActed<RememberedPlaces>>;
  /** Holds the place up for a walk: nothing for a deployed one, a checkout started in a slot for a pulled-down one. Progress arrives on `onReviewWalk`. */
  openReviewWalk: (manifestId: string, open: OpenReviewWalk) => Promise<ReviewActed<ReviewWalk>>;
  /** Stops a pulled-down walk's server and gives its slot back. Main calls this itself when the walk window closes. */
  closeReviewWalk: (manifestId: string, id: string) => Promise<ReviewActed<ReviewWalk>>;
  /** `review_walk.changed`: a held walk moved, whole. Not replayed. */
  onReviewWalk: (onWalk: (walk: ReviewWalk) => void) => () => void;
  /** Opens Armada's walk window on a walk. Closing the window closes the held walk at Fleet. */
  showReviewWalk: (shown: ShownReviewWalk) => Promise<ShownReviewWalkOutcome>;
  /** A note taken in a review's walk window that no Session holds. It stays in Armada. */
  onReviewWalkNote: (onNote: (note: ReviewWalkNote) => void) => () => void;
  /** A review's walk window closed, by the person or by Bridge, and its walk is closed at Fleet. */
  onReviewWalkClosed: (onClosed: (walkId: string) => void) => () => void;
};

export const REVIEWS_CHANNELS = {
  getReviewLists: "bridge:get-review-lists",
  readReviewPage: "bridge:read-review-page",
  readReviewFile: "bridge:read-review-file",
  readReviewPatches: "bridge:read-review-patches",
  writeReview: "bridge:write-review",
  resolveReviewThread: "bridge:resolve-review-thread",
  listReviewTools: "bridge:list-review-tools",
  saveReviewTool: "bridge:save-review-tool",
  startReviewRun: "bridge:start-review-run",
  getReviewRuns: "bridge:get-review-runs",
  /** Main to every window, not a request. */
  reviewRunChanged: "bridge:review-run-changed",
  startReviewSearch: "bridge:start-review-search",
  getReviewSearches: "bridge:get-review-searches",
  forgetReviewSearch: "bridge:forget-review-search",
  findWalkPlace: "bridge:find-walk-place",
  listWalkPlaces: "bridge:list-walk-places",
  saveWalkPlace: "bridge:save-walk-place",
  forgetWalkPlace: "bridge:forget-walk-place",
  openReviewWalk: "bridge:open-review-walk",
  closeReviewWalk: "bridge:close-review-walk",
  showReviewWalk: "bridge:show-review-walk",
  /** Main to every window, not a request. */
  reviewWalkChanged: "bridge:review-walk-changed",
  reviewWalkNote: "bridge:review-walk-note",
  reviewWalkClosed: "bridge:review-walk-closed",
} as const;
