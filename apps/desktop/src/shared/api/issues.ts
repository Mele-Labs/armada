// A repository's open issues, read through Fleet from its tracker. #91, `docs/concepts/tracker.md`.
// A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { IssueList, IssueQueries, IssueRead, Outcome, PullRequestState, SaveIssueQuery, SavedIssueQuery, WrittenIssueQuery } from "@armada/protocol";

/** What a read came to: the list as Fleet answered it, or the refusal to word. */
export type IssuesRead = { ok: true; value: IssueList } | { ok: false; outcome: Outcome };

/** One issue by reference, as Fleet answered it, or the refusal to word. */
export type IssueNumberRead = { ok: true; value: IssueRead } | { ok: false; outcome: Outcome };

/** One pull request by number, whatever its standing, or the refusal to word. */
export type PullRequestNumberRead = { ok: true; value: PullRequestState } | { ok: false; outcome: Outcome };

/** What a tracker query act came to: the thing Fleet answered with, or the refusal to word. A tracker's own words for refusing a query ride in the outcome. */
export type IssueQueryActed<T> = { ok: true; value: T } | { ok: false; outcome: Outcome };

export type IssuesApi = {
  /**
   * The open issues of the repository whose Manifest is `manifestId`. Fleet answers what it read in
   * the last five minutes; `refresh` asks the tracker again.
   */
  listIssues: (manifestId: string, refresh: boolean) => Promise<IssuesRead>;
  /** One issue of the repository by its tracker's reference (`#214`, `ABC-123`), open or closed. A reference that is no issue is Fleet's 404. */
  getIssue: (manifestId: string, reference: string) => Promise<IssueNumberRead>;
  /** Runs one query, written in the repository's tracker's language, and lists what it matches. Read from the tracker each time; no model is called. */
  searchIssues: (manifestId: string, query: string) => Promise<IssueQueryActed<IssueList>>;
  /** What the repository's tracker's query language offers, and the queries saved for it. */
  listIssueQueries: (manifestId: string) => Promise<IssueQueryActed<IssueQueries>>;
  /** One model call turns his words into a query, which Fleet runs before it answers. **This spends.** Nothing is kept. */
  writeIssueQuery: (manifestId: string, words: string) => Promise<IssueQueryActed<WrittenIssueQuery>>;
  /** Keeps a query on this Mac, or edits the saved one its `id` names. No model is called. */
  saveIssueQuery: (manifestId: string, save: SaveIssueQuery) => Promise<IssueQueryActed<SavedIssueQuery>>;
  /** Drops a saved query. Answers what is left. */
  forgetIssueQuery: (manifestId: string, id: string) => Promise<IssueQueryActed<IssueQueries>>;
  /** One pull request of the repository by number, open, merged or closed. */
  getPullRequest: (manifestId: string, number: number) => Promise<PullRequestNumberRead>;
  /**
   * Read one pull request from the forge now, rather than at the pull reading's next turn. Fleet updates its
   * ledger and merge line and tells the owning Job; answers what `getPullRequest` would.
   */
  refreshPullRequest: (manifestId: string, number: number) => Promise<PullRequestNumberRead>;
};

export const ISSUES_CHANNELS = {
  listIssues: "bridge:list-issues",
  getIssue: "bridge:get-issue",
  searchIssues: "bridge:search-issues",
  listIssueQueries: "bridge:list-issue-queries",
  writeIssueQuery: "bridge:write-issue-query",
  saveIssueQuery: "bridge:save-issue-query",
  forgetIssueQuery: "bridge:forget-issue-query",
  getPullRequest: "bridge:get-pull-request",
  refreshPullRequest: "bridge:refresh-pull-request",
} as const;
