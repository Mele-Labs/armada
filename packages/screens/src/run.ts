// A step's state and time, the run's steps while this Job's own read is out,
// and why it has none.

import type { RunTreeSkeletonStep, StepActivity } from "@armada/components";
import { STEP_STATE } from "@armada/components";
import type { StepDetail, Watched, WorkflowSummary } from "@armada/protocol";
import { span } from "./duration";

/**
 * The run before this Job's own read answers: the workflow's steps by name, in
 * order. A Job runs the workflow frozen at dispatch, so where it was edited
 * since these can differ, and the read corrects them when it lands.
 */
export function stepsAhead(workflow: WorkflowSummary | undefined): RunTreeSkeletonStep[] {
  return (workflow?.steps ?? []).map((step) => ({
    id: step.step_id,
    label: step.label,
    labelIsAnIdentifier: step.label === step.step_id || undefined,
  }));
}

/**
 * The step's state in words — the mark's accessible name, and nothing visible.
 * The word is `enum-verbs.toml`'s, through `STEP_STATE`; a state this build's
 * registry has no row for falls back to its own wire spelling.
 */
export function stateOf(step: StepDetail): string {
  return STEP_STATE[step.state]?.verb ?? step.state;
}

/**
 * How long the step took, or nothing. The rules are `rail.ts`'s and are
 * restated rather than shared because the two files draw different components
 * from the same record: a running step measures to `now`, an unstarted one
 * shows nothing, and a frozen one never measures to a clock that moves.
 */
export function took(step: StepDetail, now: number, frozen: boolean): string | undefined {
  if (step.state === "running" && !frozen) return span(step.entered_at, now) ?? undefined;
  if (step.state === "not_started" || step.entered_at === step.updated_at) return undefined;
  return span(step.entered_at, step.updated_at) ?? undefined;
}

/** Every step state the registry spells. Anything else claims nothing. */
const ACTIVITIES: readonly StepActivity[] = [
  "not_started",
  "running",
  "awaiting_human",
  "retrying",
  "advanced",
  "stopped",
];

export function activityOf(state: string): StepActivity {
  return ACTIVITIES.find((known) => known === state) ?? "not_started";
}

/**
 * Why the run has no rows, which is never the same sentence twice. Beside the
 * run it explains, out of `JobDetail.tsx` at the 900-line line.
 */
export function whyNoSteps(watched: Watched, jobId: string): string | undefined {
  if (watched.state === "read" && watched.jobId === jobId) {
    if (watched.detail.steps.length > 0) return undefined;
    // **Not yet, rather than empty.** A Job being proposed has no frozen
    // workflow at all — `job-statuses.toml` says this is the one status where
    // that is the answer — so *this Job's frozen workflow has no steps* would
    // name a workflow nobody has chosen and read as a Job that arrived broken.
    //
    // **Two sentences on that status since 30 Sep 2026**, because a proposal
    // fills in as it is written and the workflow is the first field to settle.
    // Once one has, the Job has a workflow and still has no steps, and the
    // first sentence would be false — what is missing then is the freeze, not
    // the choice, and the freeze is a press away.
    if (watched.detail.job.status === "proposing") {
      return watched.detail.job.workflow_id === ""
        ? "The proposer has not chosen a workflow yet."
        : "The proposer has chosen a workflow. Its steps freeze when you approve the dispatch.";
    }
    return "This Job's frozen workflow has no steps.";
  }
  if (watched.state === "failed" && watched.jobId === jobId) {
    return "Fleet did not answer";
  }
  return "Reading this Job.";
}
