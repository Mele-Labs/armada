// Bugs agents reported in Armada, kept in Fleet: every route, and the event that carries them whole. A slice imports
// protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { BugReport, BugReportDropped, BugReports, Outcome } from "@armada/protocol";

/** What a read or an act on a report came to: what Fleet answered with, or the refusal to word. */
export type BugReportActed<T> = { ok: true; value: T } | { ok: false; outcome: Outcome };

export type BugReportsApi = {
  /** Every report on this Mac, newest first. */
  listBugReports: () => Promise<BugReportActed<BugReports>>;
  /** Keeps a report. Answers it with `kept` set; a report that is gone is Fleet's `fleet.no_such_bug_report`. */
  keepBugReport: (id: string) => Promise<BugReportActed<BugReport>>;
  /** Deletes a report, kept or not. A second drop is Fleet's `fleet.no_such_bug_report`. */
  dropBugReport: (id: string) => Promise<BugReportActed<BugReportDropped>>;
  /** `bug_reports.changed`: every report, whole, after one was made, kept or dropped. */
  onBugReportsChanged: (onChanged: (reports: BugReports) => void) => () => void;
};

export const BUG_REPORTS_CHANNELS = {
  listBugReports: "bridge:list-bug-reports",
  keepBugReport: "bridge:keep-bug-report",
  dropBugReport: "bridge:drop-bug-report",
  /** Main to every window, not a request. */
  bugReportsChanged: "bridge:bug-reports-changed",
} as const;
