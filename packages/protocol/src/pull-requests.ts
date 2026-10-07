// A pull request by repository and number, and the acts a Session takes on
// one. `crates/ipc/src/pull_requests.rs`, `docs/concepts/session.md`. Since
// protocol 23.48.
//
// The header rules in `protocol.ts` hold here.

/** Where a pull request stands. A draft is open and not yet ready for review. */
export type PullRequestStanding = "draft" | "open" | "merged" | "closed";

/**
 * What the forge's own checks have come to. Not Armada's Checks. A repository
 * that runs nothing reads `pending`, because nothing passed.
 */
export type ForgeChecks = { state: "pending" } | { state: "passed" } | { state: "failed"; failing: string[] };

/** `get_pull_request`, and what `ready_pull_request`, `merge_pull_request_by_number` and `enable_auto_merge` answer with: the pull request as the forge shows it afterwards. */
export type PullRequestState = {
  manifest_id: string;
  number: number;
  state: PullRequestStanding;
  /** The forge will merge it when its required checks pass. */
  auto_merge: boolean;
  checks: ForgeChecks;
  title: string;
  /** The branch it is opened from. */
  branch: string;
  /** Where it is on the forge. */
  address: string;
};

/** What `review_pull_request` is asked. `POST /pull_request_reviews/:repository`. */
export type ReviewPullRequest = {
  /** The pull request's number, or its address. */
  pull_request: string;
  /** The Session the press came from. Absent records no Session. */
  session_id?: string;
};

/** What `review_pull_request` made: a Code Review Job at the approval gate. */
export type ReviewDispatched = {
  job_id: string;
  /** The pull request it reviews, as an address. */
  address: string;
  session_id?: string;
};
