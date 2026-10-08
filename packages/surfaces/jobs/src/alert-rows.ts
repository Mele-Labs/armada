// The Jobs `list_alerts` names, joined to the Trigger their Board row's bell is about.
//
// **Fleet's list names the Job and where it stopped; the Trigger is on the row** (`JobSummary.alert`),
// so a listed Job whose row carries none is waiting on something other than a Trigger and draws no row
// here. Blocked first, then waiting, each in the order Fleet sent: oldest first.

import type { AlertList, JobAlert, JobSummary } from "@armada/protocol";

export type AlertRow = { job: string; name: string; alert: JobAlert };

export function alertRowsOf(list: AlertList, jobs: readonly JobSummary[]): AlertRow[] {
  const rows: AlertRow[] = [];
  for (const one of [...list.blocked, ...list.waiting]) {
    const alert = jobs.find((job) => job.id === one.job_id)?.alert;
    if (alert !== undefined && !rows.some((row) => row.job === one.job_id)) rows.push({ job: one.job_id, name: one.handle, alert });
  }
  return rows;
}
