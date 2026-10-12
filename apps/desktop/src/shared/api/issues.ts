// A repository's open issues, read through Fleet from its tracker. #91, `docs/concepts/tracker.md`.
// A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { IssueList, Outcome } from "@armada/protocol";

/** What a read came to: the list as Fleet answered it, or the refusal to word. */
export type IssuesRead = { ok: true; value: IssueList } | { ok: false; outcome: Outcome };

export type IssuesApi = {
  /**
   * The open issues of the repository whose Manifest is `manifestId`. Fleet answers what it read in
   * the last five minutes; `refresh` asks the tracker again.
   */
  listIssues: (manifestId: string, refresh: boolean) => Promise<IssuesRead>;
};

export const ISSUES_CHANNELS = {
  listIssues: "bridge:list-issues",
} as const;
