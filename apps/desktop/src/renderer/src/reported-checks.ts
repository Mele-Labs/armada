// The Checks Fleet reports for a Job's gate and for a Drone's own runs.
//
// **A seam, not a read yet.** The manifest-wide read that serves these is being added to Fleet;
// until it lands nothing here asks for anything and the page lists the checkout's and the merge
// line's. The mock fills it through `setReportedChecks`, and the real command replaces `read`.

import type { ReportedCheck } from "@armada/screens";

let read: () => Promise<readonly ReportedCheck[]> = async () => [];

export const readReportedChecks = (): Promise<readonly ReportedCheck[]> => read();

/** For the mock only: stand in for the read with a fixture. */
export function setReportedChecks(rows: readonly ReportedCheck[]): void {
  read = async () => rows;
}
