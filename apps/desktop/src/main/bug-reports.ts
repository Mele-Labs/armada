// Bugs agents reported in Armada, as main asks Fleet for them: one request per route, and the stream's
// `bug_reports.changed`, passed on to every window. Nothing is kept here, since the event carries the list whole.

import type { BugReport, BugReportDropped, BugReports, Outcome } from "@armada/protocol";
import type { BugReportActed } from "../shared/api/bug-reports";
import { ask } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

export class BugReportsHost {
  private readonly port: () => number | null;
  private told: (reports: BugReports) => void = () => {};

  constructor(port: () => number | null) {
    this.port = port;
  }

  /** Say where changed reports go: main's, which owns the windows. */
  onChanged(told: (reports: BugReports) => void): void {
    this.told = told;
  }

  /** `bug_reports.changed`. */
  changed(reports: BugReports): void {
    this.told(reports);
  }

  /** `GET /bug_reports`. */
  list(): Promise<BugReportActed<BugReports>> {
    return this.act("GET", "");
  }

  /** `POST /bug_reports/:report_id/keep`. */
  keep(id: string): Promise<BugReportActed<BugReport>> {
    return this.act("POST", `/${encodeURIComponent(id)}/keep`);
  }

  /** `POST /bug_reports/:report_id/drop`. */
  drop(id: string): Promise<BugReportActed<BugReportDropped>> {
    return this.act("POST", `/${encodeURIComponent(id)}/drop`);
  }

  private async act<T>(method: "GET" | "POST", rest: string): Promise<BugReportActed<T>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, method, `/bug_reports${rest}`);
    return answer.ok === true ? { ok: true, value: answer.body as T } : { ok: false, outcome: answer.outcome };
  }
}
