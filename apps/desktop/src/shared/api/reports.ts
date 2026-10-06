// Reports filed against Jobs, and the counts beside them.
// A slice imports protocol and screens, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { FileReport, Outcome, Reports } from "@armada/protocol";

export type ReportsApi = {
  /**
   * Say that this Job failed in error, in your own words, and file the Job's
   * own record with it.
   *
   * **Not an act on the Job.** Nothing moves, nothing is spawned and nothing is
   * dispatched: a report is a record of what a person concluded, and an entry
   * that also moved the Job would make disagreeing with a verdict a way of
   * getting past one — which is `overrideVerdict`, a different act with a
   * different refusal.
   *
   * The sentence is required and blank is refused before the request is sent,
   * matching the 422 Fleet would give it. What comes back on the outcome is the
   * report, because the rendered record is what a person does the next thing
   * with — Armada does not file it anywhere, and says so.
   */
  fileReport: (jobId: string, filing: FileReport) => Promise<Outcome>;
  /**
   * Read every filed report and the counts beside them, or `false` to drop it.
   *
   * **Read-only, and the only read here that names no Job.** A report is about
   * a Job but does not belong to one — it survives the Job being forgotten — so
   * a listing reached through a Job would lose exactly the reports that most
   * need reading. Nothing about this files, edits or withdraws one; filing is
   * `fileReport`, on the Job it is about.
   *
   * A boolean rather than an id for that reason: there is nothing to scope it
   * to, only whether somebody is looking.
   */
  readReports: (want: boolean) => Promise<void>;
};

export type ReportsState = {
  /**
   * Every report filed, and the calibration counts, where a surface asked.
   *
   * **Not per Job, and that is the point of it.** A report outlives the Job it
   * is about — `armada clean` forgets the Job and the report stays whole — so
   * this is the one read here that no Job id scopes, and the one that would be
   * lost if it were reachable only through a Job.
   *
   * Read when the surface that draws it opens, like the folded reads under a
   * Job and for the same reason: the bodies travel with the list, so nothing
   * pays for them until somebody is reading them.
   */
  reports: Reports;
};

export const REPORTS_NOTHING_YET: ReportsState = {
  reports: { state: "none" },
};

export const REPORTS_CHANNELS = {
  fileReport: "bridge:file-report",
  readReports: "bridge:read-reports",
} as const;
