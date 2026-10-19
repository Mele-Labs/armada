// A pull request by repository and number, and the acts a Session takes on
// one. `crates/ipc/src/pull_requests.rs`, `docs/concepts/session.md`. Since
// protocol 23.48.
//
// The header rules in `protocol.ts` hold here.

import type { PullRequestOwner } from "./merge-lines";
import type { PullReviewComment, PullReviewThread } from "./pull-reviews";

/** Where a pull request stands. A draft is open and not yet ready for review. */
export type PullRequestStanding = "draft" | "open" | "merged" | "closed";

/**
 * What the forge's own checks have come to. Not Armada's Checks. A repository
 * that runs nothing reads `pending`, because nothing passed.
 */
export type ForgeChecks = { state: "pending" } | { state: "passed" } | { state: "failed"; failing: string[]; weight?: FailureWeight };

/** How much a failed check decides: with a gate named, only the gate does. Fleet refuses a merge on any of them. */
export type FailureWeight = "required" | "while_running" | "after_required";

/** `get_pull_request`, and what `ready_pull_request`, `merge_pull_request_by_number` and `enable_auto_merge` answer with: the pull request as the forge shows it afterwards. */
export type PullRequestState = {
  manifest_id: string;
  number: number;
  state: PullRequestStanding;
  /** The forge will merge it when its required checks pass. */
  auto_merge: boolean;
  /** It is in the base branch's merge queue now. */
  queued?: boolean;
  checks: ForgeChecks;
  title: string;
  /** The branch it is opened from. */
  branch: string;
  /** Where it is on the forge. */
  address: string;
  /** Its description, as Markdown. Absent where the author wrote none. */
  body?: string;
};

/** What `review_pull_request` is asked. `POST /pull_request_reviews/:repository`. */
export type ReviewPullRequest = {
  /** The pull request's number, or its address. */
  pull_request: string;
  /** The Session the press came from. Absent records no Session. */
  session_id?: string;
};

// What the pull request panel's tabs draw, and the writes its comment box makes. Read apart from
// `PullRequestState`, which rides every `session.changed`: a conversation is read when the panel
// opens, not carried. `get_pull_request_detail`.

/** Where the forge says a merge would stand against the base branch. */
export type PullRequestMergeable = "mergeable" | "conflicting" | "unknown";

/** What the reviews have come to. `required` is a review the base branch asks for and none has given. */
export type PullRequestReviewState = "approved" | "changes_requested" | "required" | "none";

/** One thing said on the pull request, in the order it was said. */
export type PullRequestEntry =
  | { kind: "comment"; author: string; at: string; body: string }
  | { kind: "review"; author: string; at: string; verdict: "approved" | "changes_requested" | "commented"; body: string };

/** One of the forge's checks, with how long it ran where the forge said. */
export type PullRequestCheckRow = {
  name: string;
  state: "passed" | "failed" | "running" | "skipped";
  /** Seconds it ran. Absent while it runs, and where the forge did not say. */
  seconds?: number;
  /** Where the forge keeps its log. */
  address?: string;
};

export type PullRequestFile = {
  path: string;
  additions: number;
  deletions: number;
  status: "added" | "modified" | "removed" | "renamed";
};

export type PullRequestCommit = {
  /** The first seven characters. */
  sha: string;
  headline: string;
  author?: string;
  at?: string;
};

/** `get_pull_request_detail`: everything the panel's tabs and facts draw that `PullRequestState` does not hold. */
export type PullRequestPage = {
  manifest_id: string;
  number: number;
  /** The branch it merges into. */
  base: string;
  author?: string;
  /** RFC 3339. */
  created_at?: string;
  additions: number;
  deletions: number;
  mergeable: PullRequestMergeable;
  review: PullRequestReviewState;
  /** Reviews that approved, counted once per reviewer's latest. */
  approvals: number;
  conversation: PullRequestEntry[];
  checks: PullRequestCheckRow[];
  files: PullRequestFile[];
  commits: PullRequestCommit[];
  /** The commit the branch is at, whole. Absent where the forge did not say. */
  head?: string;
  /** Its description, as Markdown. Absent where the author wrote none. */
  body?: string;
  /** The conversations on its diff, resolved ones too: the forge's first 100. Absent when there are none. */
  threads?: PullReviewThread[];
  /** Who registered it in the central store, as the merge line's open pull requests carry it. Absent as `HubPullRequest.owner` is. */
  owner?: PullRequestOwner;
};

/**
 * `get_pull_request_file`: one file of a pull request, read when a person opens it from the Files
 * tab and carried nowhere. `patch` is the forge's hunks with no `diff --git` header. Where the forge
 * sent none (binary, or too large) `patch` is absent and `said` is Fleet's sentence.
 */
export type PullRequestFilePatch = {
  manifest_id: string;
  number: number;
  path: string;
  patch?: string;
  said?: string;
};

/**
 * `get_pull_request_patches`: every file of a pull request, in the forge's order, each as
 * `get_pull_request_file` answers it, from one forge read. `GET /pull_request_patches/:repository/:number`.
 */
export type PullRequestFilePatches = {
  files: readonly ({ path: string } & PullRequestFilePatch)[];
};

/** What the panel's comment box writes. `POST /pull_request_writes/:repository`. */
export type WritePullRequest = {
  /** The pull request's number. */
  pull_request: number;
  kind: "comment" | "approve" | "request_changes";
  /** Markdown. Required for `comment` and `request_changes`; optional for `approve`. */
  body?: string;
  /** The comments of the review, posted with it as one review. With any, `body` may be empty. */
  comments?: PullReviewComment[];
  /** The commit the person read, whole: their line comments land on it. Absent reviews the branch as it stands. */
  commit?: string;
  /** The Session the press came from. Absent records no Session. */
  session_id?: string;
};
