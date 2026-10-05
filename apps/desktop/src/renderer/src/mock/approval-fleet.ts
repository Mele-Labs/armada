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
  Outcome,
  PolicyOverrides,
  StepDetail,
  StepTuning,
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
  if ((body?.tuning ?? []).length > 0) {
    approved.steps = approved.steps.map((step) => {
      const tuned = body?.tuning?.find((one) => one.step_id === step.step_id);
      return tuned === undefined ? step : tunedAs(step, tuned);
    });
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

/** A refusal, in Fleet's own code and words. */
const refusedAs = (code: string, message: string): Outcome => ({
  ok: false,
  why: "refused",
  error: { code, message, run_id: "mock", fields: {}, chain: [] },
});

/**
 * What Fleet refuses in `tuning` (23.20), keeping nothing: a step the workflow
 * lacks, one tuned twice, a blank model, `judges: 0`, and `judges` on a step
 * with no Judge.
 */
export function tuningRefusal(detail: JobDetail, body: ApproveDispatch | undefined): Outcome | undefined {
  const refuse = (why: string): Outcome => refusedAs("fleet.unacceptable_proposal", why);
  const seen = new Set<string>();
  for (const tuned of body?.tuning ?? []) {
    const step = detail.steps.find((one) => one.step_id === tuned.step_id);
    if (step === undefined) return refuse(`\`${tuned.step_id}\` is not a step of this workflow`);
    if (seen.has(tuned.step_id)) return refuse(`\`${tuned.step_id}\` is tuned twice`);
    seen.add(tuned.step_id);
    if (tuned.model !== undefined && tuned.model.trim() === "") return refuse("a model cannot be blank");
    if (tuned.judges === 0) return refuse("`judges` cannot be zero");
    if (tuned.judges !== undefined && (step.judge_checks ?? []).length === 0) {
      return refuse(`\`${tuned.step_id}\` has no Judge`);
    }
  }
  return undefined;
}

/**
 * The step as tuned, as `get_job` reads it back: the Judge's panel and the
 * Checks the step still runs. The model, effort and words are not served back.
 */
function tunedAs(step: StepDetail, tuned: StepTuning): StepDetail {
  const off = tuned.checks_off ?? [];
  return {
    ...step,
    ...(tuned.judges === undefined
      ? {}
      : { judge_checks: (step.judge_checks ?? []).map((one) => ({ ...one, panel_size: tuned.judges! })) }),
    ...(off.length === 0 ? {} : { checks: (step.checks ?? []).filter((one) => !off.includes(one.name ?? one.kind)) }),
  };
}

/**
 * `set_landing_target` (23.22): once, on an approved Job landing in the base.
 * Refused blank (422) and where the approval set it or the work is out (409).
 */
export function landingTargetSet(detail: JobDetail, target: string): JobDetail | Outcome {
  if (target.trim() === "") return refusedAs("fleet.landing_target_blank", "a landing target cannot be blank");
  if (detail.job.status === "awaiting_approval" || detail.landing?.target !== undefined) {
    return refusedAs("fleet.landing_target_settled", "this Job's landing target is already settled");
  }
  return {
    ...detail,
    landing: { pr_mode: "ready", ...detail.landing, target: target.trim() },
  };
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
