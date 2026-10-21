// A repository's open issues, read through Fleet from its tracker. #91, `docs/concepts/tracker.md`.
// A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { IssueList, IssueRead, Outcome, PullRequestState } from "@armada/protocol";

/** What a read came to: the list as Fleet answered it, or the refusal to word. */
export type IssuesRead = { ok: true; value: IssueList } | { ok: false; outcome: Outcome };

/** One issue by reference, as Fleet answered it, or the refusal to word. */
export type IssueNumberRead = { ok: true; value: IssueRead } | { ok: false; outcome: Outcome };

/** One pull request by number, whatever its standing, or the refusal to word. */
export type PullRequestNumberRead = { ok: true; value: PullRequestState } | { ok: false; outcome: Outcome };

export type IssuesApi = {
  /**
   * The open issues of the repository whose Manifest is `manifestId`. Fleet answers what it read in
   * the last five minutes; `refresh` asks the tracker again.
   */
  listIssues: (manifestId: string, refresh: boolean) => Promise<IssuesRead>;
  /** One issue of the repository by its tracker's reference (`#214`, `ABC-123`), open or closed. A reference that is no issue is Fleet's 404. */
  getIssue: (manifestId: string, reference: string) => Promise<IssueNumberRead>;
  /** One pull request of the repository by number, open, merged or closed. */
  getPullRequest: (manifestId: string, number: number) => Promise<PullRequestNumberRead>;
  /**
   * Read one pull request from the forge now, rather than at the pull watch's next turn. Fleet updates its
   * ledger and merge line and tells the owning Job; answers what `getPullRequest` would.
   */
  refreshPullRequest: (manifestId: string, number: number) => Promise<PullRequestNumberRead>;
};

export const ISSUES_CHANNELS = {
  listIssues: "bridge:list-issues",
  getIssue: "bridge:get-issue",
  getPullRequest: "bridge:get-pull-request",
  refreshPullRequest: "bridge:refresh-pull-request",
} as const;
