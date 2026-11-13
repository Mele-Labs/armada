// What Fleet remembers of a pull request for the person between visits, and what it works out from it: the head he
// last left it at, the files he marked viewed, where each row stands in his inbox, the change since he looked, and his
// named Reviews layouts. `crates/ipc/src/pull_review_memory.rs`, `docs/concepts/pull-review.md`, *Since you looked*.
//
// The header rules in `protocol.ts` hold here.

import type { PullRequestFilePatch } from "./pull-requests";

/** The head a person last left a pull request at. */
export type PullSeen = {
  /** The commit, whole. */
  head: string;
  /** RFC 3339, when he left it. */
  at: string;
};

/** What his own latest submitted review said. A pending review is not one. */
export type MyReviewSaid = "approved" | "changes_requested" | "commented";

/** His own latest submitted review of a pull request. */
export type PullMyReview = {
  said: MyReviewSaid;
  /** The commit it was written on, whole. Absent where the forge did not say. */
  commit?: string;
};

/**
 * Where a pull request stands in his inbox. The first that holds wins, in this order: the head moved since he looked,
 * his latest review approved, it commented or asked for changes, anything else.
 */
export type PullInbox =
  | "changed_since_you_looked"
  | "done"
  | "waiting_on_author"
  | "needs_first_look";

/**
 * `mark_pull_request_seen`'s body (`POST /pull_request_seen/:repository`), answered with the `PullSeen` kept. Sent when
 * he leaves the pull request or presses Mark as seen, never on open.
 */
export type MarkPullSeen = {
  number: number;
  head: string;
};

/** A file he marked viewed, as the page carries it. `still` is false once the file changed since. */
export type PullViewedFile = {
  path: string;
  /** The commit he viewed it at, whole. */
  commit: string;
  /** RFC 3339. */
  viewed_at: string;
  still: boolean;
};

/** `mark_file_viewed`'s body (`POST /pull_request_viewed/:repository`), answered with the `PullViewedFile` kept. */
export type MarkFileViewed = {
  number: number;
  path: string;
  /** The head he read it at, whole. */
  commit: string;
  /** `PullRequestFilePatch.blob` as he read it. Absent, the mark goes stale once the head moves at all. */
  blob?: string;
};

/** `unmark_file_viewed`'s body and answer (`POST /pull_request_viewed/:repository/unmark`). */
export type UnmarkFileViewed = {
  number: number;
  path: string;
};

/** `get_pull_request_compare`'s query (`GET /pull_request_compares/:repository/:number`). */
export type PullRequestCompareOf = {
  /** His seen head, whole. */
  since: string;
  /** The page's head, whole. */
  head: string;
};

/**
 * `get_pull_request_compare`: what changed from `since` to `head`. The forge compares from the two commits' merge base,
 * so after a rebase `files` can hold changes from the target branch. `base_gone`: the forge no longer has `since`.
 */
export type PullRequestCompare =
  | {
      outcome: "patches";
      since: string;
      head: string;
      files: PullRequestFilePatch[];
    }
  | {
      outcome: "base_gone";
      since: string;
      head: string;
    };

/** A named Reviews layout. `arrangement` is the pane tree as JSON text, `{ root, focus, next, threadWas? }`. */
export type ReviewLayout = {
  name: string;
  arrangement: string;
  /** Fleet's own, which cannot be saved over or deleted. */
  shipped?: boolean;
};

/** `list_review_layouts` (`GET /review_layouts`), and the answer of every write on them. */
export type ReviewLayouts = {
  /** Fleet's `Default` first. */
  layouts: ReviewLayout[];
  /** The one every pull request opens on. */
  picked: string;
};

/** `save_review_layout`'s body (`POST /review_layouts`). A name already kept is replaced. */
export type SaveReviewLayout = {
  name: string;
  arrangement: string;
};

/** `delete_review_layout`'s body (`POST /review_layouts/delete`) and `pick_review_layout`'s (`POST /review_layouts/pick`). */
export type NamedReviewLayout = {
  name: string;
};

/** The name of Fleet's own layout. */
export const DEFAULT_REVIEW_LAYOUT = "Default";
