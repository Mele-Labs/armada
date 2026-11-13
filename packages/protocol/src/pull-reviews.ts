// The pull requests waiting on a person, as four lists read from the forge, and the acts a review makes on
// one. `crates/ipc/src/pull_reviews.rs`, `docs/concepts/pull-review.md`.
//
// The header rules in `protocol.ts` hold here.

import type { PullRequestReviewState } from "./pull-requests";
import type { PullInbox, PullMyReview, PullSeen } from "./pull-review-memory";
import type { PullReviewHolder } from "./review-runs";

/** The four lists. A pull request can be on several. */
export type PullReviewList = "requested" | "team" | "assigned" | "mine";

/** What a row's forge checks have come to. Not Armada's Checks. */
export type PullReviewChecks = "passed" | "failed" | "running";

/** One pull request on one list: what a row of the list draws. */
export type PullReviewRow = {
  manifest_id: string;
  /** `owner/name`, as the forge names the repository. */
  repository: string;
  number: number;
  title: string;
  /** The branch it is opened from. */
  branch: string;
  /** Where it is on the forge. */
  address: string;
  draft: boolean;
  checks: PullReviewChecks;
  review: PullRequestReviewState;
  /** The commit the branch is at, whole. */
  head: string;
  author: string;
  /** RFC 3339. */
  updated_at: string;
  /** The team asked, as the forge names it, on the `team` list only. */
  team?: string;
  /** The Session of the person's that holds it, where one does. Absent, and a note on it stays in Armada. */
  holder?: PullReviewHolder;
  /** The head the person last left it at. Absent where he never has. */
  seen?: PullSeen;
  /** His own latest submitted review of it. Absent where he has none. */
  my_review?: PullMyReview;
  /** Where it stands in his inbox, worked out by Fleet. Absent from a Fleet before it, read as `needs_first_look`. */
  inbox?: PullInbox;
};

/** `get_review_lists`: read when the Reviews tab opens and on Refresh, never polled. At most `MOST_PER_LIST` a list. */
export type PullReviewLists = {
  /** Who the person is on the forge. Their own words on a thread are signed so. */
  me: string;
  /** RFC 3339, when Fleet read the lists. */
  fetched_at: string;
  requested: PullReviewRow[];
  team: PullReviewRow[];
  assigned: PullReviewRow[];
  mine: PullReviewRow[];
  /** The lists the forge would not answer. Their rows are empty and say nothing: keep what was last held. */
  silent?: PullReviewList[];
};

/** The forge is asked for exactly this many a list, so a list at the cap has more behind it. */
export const MOST_PER_LIST = 50;

/** Which side of the diff a line is on: the base (`left`) or the head (`right`). */
export type DiffSide = "left" | "right";

/** One thing said on a review thread. */
export type PullReviewThreadComment = {
  id: string;
  author: string;
  /** Whether an app wrote it. Absent for a person. */
  bot?: boolean;
  /** RFC 3339. */
  at: string;
  /** When it was last edited, RFC 3339. Absent when it never was. */
  edited_at?: string;
  /** Markdown. */
  body: string;
};

/** A conversation on a line of the diff, or on a file, as the forge holds it. */
export type PullReviewThread = {
  /** The forge's own id, which a reply and a resolve name it by. */
  id: string;
  path: string;
  /** The line in the file on `side`. Absent where the thread is on the file as a whole, or the line has since changed under it. */
  line?: number;
  side: DiffSide;
  resolved: boolean;
  comments: PullReviewThreadComment[];
};

/** One comment of the review a person submits. They leave together, with the review. */
export type PullReviewComment =
  | {
      kind: "line";
      path: string;
      /** The last line of the range, or the only line. */
      line: number;
      side: DiffSide;
      body: string;
      /** The first line of a range on `side`, before `line`. Absent is one line. */
      start_line?: number;
    }
  | {
      kind: "file";
      path: string;
      body: string;
    }
  | {
      kind: "reply";
      thread: string;
      body: string;
    };

/** `resolve_review_thread`'s body. `POST /review_threads/:repository`. */
export type ResolvePullThread = {
  thread: string;
  /** `false` opens it again. */
  resolved: boolean;
};

/** What `resolve_review_thread` answers: the thread as the forge now holds it. */
export type PullThreadResolved = {
  thread: string;
  resolved: boolean;
};
