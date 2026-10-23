// A repository's open issues, as Fleet read them from its tracker. Mirrored by hand from
// `crates/ipc/src/issues.rs`. #91, `docs/concepts/tracker.md`.
//
// The header rules in `protocol.ts` hold here.

/** One open issue. Every string is the tracker's own, unedited. */
export type TrackedIssue = {
  /** The tracker's own name for it, as a person writes it: `#123` on the forge, `ABC-123` on Jira. Drawn as it is. */
  reference: string;
  title: string;
  /** The tracker's address for it: what a dispatch from it sends as the request. */
  url: string;
  labels: string[];
  assignees: string[];
  updated_at: string;
  /** The start of its body, whitespace folded. */
  excerpt: string;
  /** The tracker says a pull request already closes it. */
  linked_pull_request: boolean;
  /** Who opened it, by login. Absent where the tracker did not say. */
  author?: string;
  /** Open on a plain list; a search may answer closed ones too. */
  state: "open" | "closed";
  /** Everything else the tracker said of it, under its own names. A field with no values is left out. */
  fields: IssueField[];
  /** The Jobs holding this issue on the ledger, by id, whichever tracker it is on. Join them with the Jobs already held. */
  holders: string[];
};

/** One of an issue's fields that Armada has no slot for, under the tracker's own name for it. */
export type IssueField = {
  name: string;
  /** Each a display string. */
  values: string[];
};

/** `GET /issues/:repository`. `stale` is why an older list was answered: the read just tried failed. */
export type IssueList = {
  manifest_id: string;
  fetched_at: string;
  issues: TrackedIssue[];
  stale?: string;
};

/** `get_issue` (`GET /issues/:repository/:reference`): one issue by reference, open or closed. 404 `fleet.no_such_issue`, 500 `fleet.issues_unreadable`. */
export type IssueRead = {
  /** As `TrackedIssue.reference`. */
  reference: string;
  title: string;
  /** The tracker's address for it. */
  url: string;
  state: "open" | "closed";
  /** The whole body, markdown as written. `TrackedIssue.excerpt` is its start. */
  body: string;
  /** Who opened it, by login. Empty where the account is gone. */
  author: string;
  /** RFC 3339. */
  opened_at: string;
  labels: string[];
  assignees: string[];
  /** Oldest first. The tracker's first hundred. */
  comments: IssueComment[];
  /** The pull requests that close it or name it, each once. */
  linked_pull_requests: LinkedPullRequest[];
  /** As `TrackedIssue.fields`. */
  fields: IssueField[];
  /** As `TrackedIssue.holders`. */
  holders: string[];
};

/** One comment on an issue. */
export type IssueComment = {
  /** By login. Empty where the account is gone. */
  author: string;
  /** RFC 3339. */
  at: string;
  body: string;
};

/** A pull request that closes an issue or names it. */
export type LinkedPullRequest = {
  number: number;
  title: string;
  url: string;
  state: "open" | "merged" | "closed";
};
