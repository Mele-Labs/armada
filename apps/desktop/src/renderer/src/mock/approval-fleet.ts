// What the mock's Fleet does with an approval body and an `edit_job`, as
// `crates/fleet/src/approving.rs` does it (23.8): the words, the gates and the
// criteria are rewritten on the record and the rest kept beside the Job.

import type {
  ApproveDispatch,
  Criterion,
  CriterionWritten,
  EditJob,
  GateChoice,
  JobDetail,
  PolicyOverrides,
  StepDetail,
} from "@armada/protocol";
import type { WaveJobView } from "@armada/screens/src/draft/wave";

/** The Job one press later: `queued`, carrying what the body set, approved `at`. */
export function approvedAs(detail: JobDetail, body: ApproveDispatch | undefined, at: string): JobDetail {
  const approved: JobDetail = {
    ...edited(detail, body ?? {}),
    approved_at: at,
  };
  approved.job = {
    ...approved.job,
    status: "queued",
    ...(body?.workflow_id === undefined ? {} : { workflow_id: body.workflow_id }),
  };
  const gates = body?.gates ?? [];
  if (gates.length > 0) {
    approved.steps = approved.steps.map((step) => {
      const choice = gates.find((one) => one.step_id === step.step_id);
      return choice === undefined ? step : gatedAs(step, choice);
    });
    const overrides = overridesOf(detail.steps, gates);
    if (overrides !== undefined) approved.policy_overrides = overrides;
  }
  if (body?.tiers !== undefined) approved.tiers = body.tiers;
  // The body is the whole proposal, so a cap left out is the machine's.
  if (body?.drone_cap === undefined) delete approved.drone_cap;
  else approved.drone_cap = body.drone_cap;
  const landing = body?.landing;
  if (landing !== undefined) {
    approved.landing = {
      ...(landing.target === undefined ? {} : { target: landing.target }),
      ...(landing.from_ref === undefined ? {} : { from_ref: landing.from_ref }),
      pr_mode: landing.pr_mode ?? "ready",
    };
  }
  return approved;
}

/** A proposal's words saved without releasing it — `edit_job`. */
export function edited(detail: JobDetail, edit: EditJob): JobDetail {
  return {
    ...detail,
    job: { ...detail.job, ...(edit.title === undefined ? {} : { title: edit.title }) },
    ...(edit.facts === undefined ? {} : { facts: edit.facts }),
    ...(edit.criteria === undefined
      ? {}
      : { acceptance_criteria: criteriaAs(detail.acceptance_criteria, edit.criteria) }),
  };
}

/** The same edit on the wave's panel, which draws the mock's own copy of the Job. */
export function waveJobEdited(job: WaveJobView, edit: EditJob): WaveJobView {
  return {
    ...job,
    ...(edit.title === undefined ? {} : { title: edit.title }),
    ...(edit.facts === undefined ? {} : { facts: edit.facts }),
    ...(edit.criteria === undefined ? {} : { criteria: criteriaAs(job.criteria ?? [], edit.criteria) }),
  };
}

/**
 * The criteria as written. A line with an id keeps its origin and when its
 * issue moved; one without is minted the next `c<n>`, and is the person's.
 */
function criteriaAs(held: readonly Criterion[], written: readonly CriterionWritten[]): Criterion[] {
  let next = held.reduce((most, one) => Math.max(most, Number(one.criterion_id.slice(1)) || 0), 0);
  return written.map((line) => {
    const was = held.find((one) => one.criterion_id === line.criterion_id);
    if (was !== undefined) return { ...was, text: line.text, source: line.source };
    next += 1;
    return { criterion_id: `c${next}`, text: line.text, source: line.source, origin: { kind: "person" } };
  });
}

/**
 * One step's gate as the approval set it. **An overridden step keeps deferring
 * on the record** and carries the override; any other takes its boxes, and an
 * unticked tier drops what it would have run.
 */
function gatedAs(step: StepDetail, choice: GateChoice): StepDetail {
  if (step.advance_gate?.startsWith("manifest_rule:") === true) {
    return { ...step, overridden: choice.overridden === true };
  }
  return {
    ...step,
    advance_gate: choice.you ? "human_always" : choice.judge ? "auto_if_judge_passes" : "auto",
    ...(choice.checks ? {} : { checks: [] }),
    ...(choice.judge ? {} : { judge_checks: [] }),
  };
}

/** What each override says in place of the repository, in `armada.yml`'s words. */
function overridesOf(steps: readonly StepDetail[], gates: readonly GateChoice[]): PolicyOverrides | undefined {
  const overrides: PolicyOverrides = {};
  for (const choice of gates) {
    if (choice.overridden !== true) continue;
    const gate = steps.find((step) => step.step_id === choice.step_id)?.advance_gate;
    if (gate === "manifest_rule:auto_merge") overrides.auto_merge = choice.you ? "never" : "checks-pass";
    if (gate === "manifest_rule:review_gate") {
      overrides.review_gate = choice.you ? "human_always" : "auto_if_judge_passes";
    }
  }
  return Object.keys(overrides).length === 0 ? undefined : overrides;
}
