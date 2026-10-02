// The owner's Job 2 **just before it landed**: at `handoff`, waiting on him to
// review it. Built from what Fleet served on 2 Oct 2026, never by hand —
// `job-2-landed.json` is `GET /jobs/2` and `job-2-evidence.json` is
// `GET /jobs/2/evidence`, both unedited. For #1680's three arrangements of The
// Job's record.
//
// **What is moved back, and only that.** The Job's status and the handoff
// step's, the step's last attempt left open, and what landing wrote: `ended_at`,
// `reclaimed_at`, `landed`. The files are pull request #1750's, in the words
// Fleet's own review spelled them. Fleet sends no pull request detail for this
// Job, so the record draws its number and nothing it would have to invent.

import type { ChangedFile, JobDetail, JobSummary, StepDetail, Submitted } from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";
import { recorded } from "@armada/screens/src/fixtures/recorded";

import served from "./job-2-landed.json";
import evidence from "./job-2-evidence.json";

/** When the handoff step asked for a person: its attempt's own `ended_at`. */
const ASKED_AT = "2026-10-01T21:23:46.361Z";

/** Pull request #1750's files, as Fleet's review of Job 2 lists them. */
const FILES: ChangedFile[] = [
  { path: "docs/contracts/design-system.md", change: "modified" },
  { path: "packages/components/src/compositions/GuideFigure/GuideFigure.css", change: "modified" },
  { path: "packages/components/src/compositions/GuideFigure/GuideFigure.stories.tsx", change: "modified" },
  { path: "packages/components/src/compositions/GuideFigure/GuideFigure.tsx", change: "modified" },
  { path: "packages/components/src/guides/008-what-does-the-progress-bar-show.ts", change: "deleted" },
  { path: "packages/components/src/guides/020-what-if-a-step-changes-a-file-it-never-said-it-would.ts", change: "deleted" },
  { path: "packages/components/src/guides/guide.ts", change: "modified" },
  { path: "packages/components/src/guides/index.ts", change: "modified" },
  { path: "packages/screens/src/guides.test.ts", change: "modified" },
  { path: "packages/screens/src/InsideAJob.tsx", change: "modified" },
  { path: "xtask/src/main.rs", change: "modified" },
  { path: "xtask/src/rules_guides.rs", change: "added" },
  { path: "xtask/src/rules_guides/tests.rs", change: "added" },
];

/** The handoff step as it stood asking: its one attempt open, no verdict yet. */
function asking(step: StepDetail): StepDetail {
  if (step.step_id !== "handoff") return step;
  const { last_verdict: _verdict, ...rest } = step;
  return {
    ...rest,
    state: "awaiting_human",
    attempts: step.attempts.map(({ ended_at: _ended, ...attempt }) => attempt),
    updated_at: ASKED_AT,
  };
}

/**
 * Job 2 at its review gate, on `id` and `handle` — one copy per arrangement, so
 * a walk can open each in turn. Every read that names its Job is moved onto `id`.
 */
export function job2AtReview(id: string = served.job.id, handle: string = served.job.handle): JobFixture {
  const base = recorded("done-worktree-given-back");
  const landed = served as unknown as JobDetail;
  const { ended_at: _ended, reclaimed_at: _reclaimed, ...rest } = landed.job;
  const job: JobSummary = { ...rest, id, handle, status: "awaiting_review", current_step_id: "handoff" };
  const { landed: _landed, ...delivery } = landed.delivery ?? {};
  const detail: JobDetail = { ...landed, job, delivery, steps: landed.steps.map(asking) };
  if (base.watched.state !== "read") return base;
  const moved = <Read extends { state: string }>(read: Read): Read =>
    "jobId" in read ? { ...read, jobId: id } : read;
  const pullRequest = delivery.pull_request ?? "";
  return {
    ...base,
    name: "Job 2, at its review gate, as Fleet served it",
    job,
    watched: { ...moved(base.watched), detail },
    observed: moved(base.observed),
    journalled: moved(base.journalled),
    resources: moved(base.resources),
    history: base.history === undefined ? undefined : moved(base.history),
    recorded: {
      footprint: moved(base.recorded.footprint),
      handed: moved(base.recorded.handed),
      evidence: { state: "read", jobId: id, steps: evidence.steps as Submitted[] },
      diff: {
        state: "read",
        jobId: id,
        work: { files: FILES, measured_from: "main", measured_whole: true, plan_declared: true },
      },
      remarks: {
        state: "read",
        jobId: id,
        review: { job_id: id, pull_request: pullRequest, remarks: [] },
      },
    },
    // A few minutes after it asked.
    now: Date.parse("2026-10-01T21:30:00Z"),
  };
}
