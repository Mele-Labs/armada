// A repository's open issues, as Fleet read them from its tracker. Mirrored by hand from
// `crates/ipc/src/issues.rs`. #91, `docs/concepts/tracker.md`.
//
// The header rules in `protocol.ts` hold here.

/** One open issue. Every string is the tracker's own, unedited. */
export type TrackedIssue = {
  number: number;
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
  /** A Job dispatched from it that has not ended. Absent where there is none. */
  job_id?: string;
};

/** `GET /issues/:repository`. `stale` is why an older list was answered: the read just tried failed. */
export type IssueList = {
  manifest_id: string;
  fetched_at: string;
  issues: TrackedIssue[];
  stale?: string;
};

/** `get_issue` (`GET /issues/:repository/:number`): one issue by number, open or closed. 404 `fleet.no_such_issue`, 500 `fleet.issues_unreadable`. */
export type IssueRead = {
  number: number;
  title: string;
  /** The tracker's address for it. */
  url: string;
  state: "open" | "closed";
};
