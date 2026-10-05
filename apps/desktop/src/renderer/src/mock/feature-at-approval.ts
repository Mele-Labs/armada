// A feature Job at its dispatch gate, for the approval canvas (prototype): the
// request read from an issue, its criteria and branches as
// `proposal-from-an-issue.ts` serves them, on `feature.json`'s four steps.
//
// **The steps are the workflow's own** (`featureWorkflow`, transcribed from
// `.armada/workflows/feature.json`), frozen as Fleet freezes them at the
// proposal, and `bug` and `refactor` are served beside it so the canvas's
// Start node has somewhere else to go.

import type { StepDetail } from "@armada/protocol";
import { featureWorkflow } from "@armada/screens/src/fixtures/build/arc-base";
import { watchedRead } from "@armada/screens/src/fixtures/build/base";
import { prototypeWorkflow } from "@armada/screens/src/fixtures/build/kinds-workflows";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { proposalFromAnIssue } from "./proposal-from-an-issue";

export function featureAtApproval(): JobFixture {
  const base = proposalFromAnIssue();
  if (base.watched.state !== "read") return base;
  const created = "2026-10-01T18:29:37.596Z";
  const feature = { ...featureWorkflow(), manifest_id: base.job.owner_manifest_id };
  const steps: StepDetail[] = feature.steps.map((step, ordinal) => ({
    step_id: step.step_id,
    label: step.label,
    ordinal,
    state: "not_started",
    checks: step.checks,
    check_runs: [],
    judge_checks: step.judge_checks,
    advance_gate: step.advance_gate,
    delivers: step.delivers,
    // `feature.json` declares a Drone per task on `implement`, as Fleet serves it since 23.1.
    ...(step.drone_per_task === true ? { drone_per_task: true } : {}),
    overridden: false,
    judged: [],
    flagged: [],
    attempts: [],
    verdicts: [],
    entered_at: created,
    updated_at: created,
  }));
  const job = {
    ...base.job,
    handle: "1-guide-catalogue-validation",
    title: "Add guide validation to the catalogue",
    workflow_id: "feature",
  };
  return {
    ...base,
    name: "awaiting_approval — a feature Job, drawn as the run it will be",
    job,
    watched: watchedRead({ ...base.watched.detail, job, steps }),
    // `prototype` beside them, whose Frame step is one a person meets for the first time.
    workflows: [
      feature,
      ...base.workflows.filter((one) => one.id !== "feature"),
      { ...prototypeWorkflow(), manifest_id: base.job.owner_manifest_id },
    ],
  };
}
