// The owner's Job 2, finished and merged, **as `GET /jobs/2` served it on
// 1 Oct 2026** — the detail is `job-2-landed.json`, unedited.
//
// Fleet sends no groups, so Bridge draws one per task, and all four tasks are
// still `open` on the wire though the Job merged (#1752). The finished-Job
// board's group rows overlapped on this read and on no mock shaped by hand.

import type { JobDetail, JobSummary } from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";
import { recorded } from "@armada/screens/src/fixtures/recorded";

import served from "./job-2-landed.json";

export function job2Landed(): JobFixture {
  const base = recorded("done-worktree-given-back");
  const detail = served as unknown as JobDetail;
  // The Board's row is the detail's own, plus `landed`, which only the row carries.
  const job: JobSummary = { ...detail.job, landed: "merged" };
  if (base.watched.state !== "read") return base;
  // Every read that names its Job is moved onto this one, or the detail draws
  // the recording's reads as another Job's.
  const moved = <Read extends { state: string }>(read: Read): Read =>
    "jobId" in read ? { ...read, jobId: job.id } : read;
  return {
    ...base,
    name: "Job 2, merged, as Fleet served it",
    job,
    watched: { ...moved(base.watched), detail },
    observed: moved(base.observed),
    journalled: moved(base.journalled),
    resources: moved(base.resources),
    history: base.history === undefined ? undefined : moved(base.history),
    recorded: {
      footprint: moved(base.recorded.footprint),
      handed: moved(base.recorded.handed),
      evidence: moved(base.recorded.evidence),
      diff: moved(base.recorded.diff),
      remarks: moved(base.recorded.remarks),
    },
  };
}
