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

/** `search_issues` (`POST /issue_queries/:repository/search`): a query in the tracker's own language, run once. Answers `IssueList`. 422 `fleet.issue_query_refused` carries the tracker's words. */
export type SearchIssues = {
  query: string;
};

/** What a person needs of a tracker's query language: its name, the two queries every repository starts from, and the fields a row can show. */
export type IssueQueryGuide = {
  /** As a person would say it: `JQL`. */
  language: string;
  /** The open issues, in the tracker's own language. */
  open: string;
  /** The open issues assigned to whoever the tracker's credentials are. */
  assigned_to_me: string;
  /** The tracker's field names a row can show. */
  fields: string[];
};

/** One query a person saved, on this Mac. */
export type SavedIssueQuery = {
  id: string;
  name: string;
  /** What the person said, which the query was written from. Empty for one typed by hand. */
  words: string;
  /** In the tracker's own language. Shown to the person, who may edit it. */
  query: string;
  /** The tracker's field names a row shows, in order. */
  row_fields: string[];
};

/** `list_issue_queries` (`GET /issue_queries/:repository`). */
export type IssueQueries = {
  manifest_id: string;
  /** Absent where the tracker cannot be read, and then `refused` says why. */
  guide?: IssueQueryGuide;
  /** Why the tracker cannot be read, in its own words. Offer to teach Armada this tracker from it. */
  refused?: string;
  /** Oldest first. Listed whether or not the tracker can be read. */
  saved: SavedIssueQuery[];
};

/** `write_issue_query` (`POST /issue_queries/:repository/write`). */
export type WriteIssueQuery = {
  words: string;
};

/** `write_issue_query`'s answer: the query, already run. Nothing is kept until `save_issue_query`. */
export type WrittenIssueQuery = {
  query: string;
  row_fields: string[];
  /** What the tracker answered the query, in `list_issues`' shape. */
  issues: TrackedIssue[];
  /** What the one model call cost, in millionths of a dollar. Absent where the call ended without saying. */
  cost_micros?: number;
};

/** `save_issue_query` (`POST /issue_queries/:repository`). With an `id` it edits that query. Answers `SavedIssueQuery`. */
export type SaveIssueQuery = {
  id?: string;
  name: string;
  words: string;
  query: string;
  row_fields: string[];
};
