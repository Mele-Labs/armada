// Bug reports: a fault in Armada an agent reported, kept on this Mac and posted nowhere.
// Hand-mirrored from `crates/ipc/src/bug_reports.rs`.
//
// The header rules in `protocol.ts` hold here. Whether the owner has read a report is Bridge's own and is not on the wire.

/** `report_bug`: what went wrong in Armada, and how to see it again. An agent sends it; Bridge does not. `POST /sessions/report_bug`. */
export type ReportBug = {
  /** One line naming the fault. */
  title: string;
  /** What went wrong: what the agent did or expected, and what Armada did instead. */
  what: string;
  /** The steps that led to it, one line each. */
  steps: string[];
  /** The exact error lines, as they were printed. */
  errors: string[];
  /** Paths of log files that hold more. */
  logs: string[];
  session_id?: string;
};

/** What `report_bug` kept. */
export type BugReported = { report_id: string };

/** One kept report, as `list_bug_reports` and `bug_reports.changed` carry it. */
export type BugReport = {
  id: string;
  /** The Job that reported it. It may be gone. */
  session_id: string;
  /** The Job's title when it reported; empty where it had none. */
  job_title: string;
  title: string;
  what: string;
  steps: string[];
  errors: string[];
  logs: string[];
  at: string;
  /** When the owner kept it. Absent is not kept. */
  kept?: string;
  /** The pull request that fixed it: one the Job started on it held, merged. Absent while the report stands open. */
  fixed?: BugFixed;
};

/** The merge that fixed a report. Set once, by the first. */
export type BugFixed = {
  /** When the pull request merged. */
  at: string;
  pull_request: number;
  title: string;
  url: string;
};

/** `list_bug_reports` (`GET /bug_reports`): every report on this Mac, newest first. */
export type BugReports = { reports: BugReport[] };

/** `bug_reports.changed`: every report now, whole, after one was made, kept or dropped. */
export type BugReportsChanged = BugReports;

/** `keep_bug_report` (`POST /bug_reports/:report_id/keep`, no body) answers the `BugReport`, `kept` set. 404 `fleet.no_such_bug_report`. */

/** `drop_bug_report` (`POST /bug_reports/:report_id/drop`, no body): the report is deleted. 404 `fleet.no_such_bug_report`, a second drop too. */
export type BugReportDropped = { id: string };
