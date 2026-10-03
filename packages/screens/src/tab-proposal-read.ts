// What the classifying screen says, read out of the draft a moment carries.
//
// **The reading is here and the open state is `tab-proposal.tsx`**, which is
// how every other destination is arranged. What is unusual is that this one is
// also *editable*: a person's changes are held by the screen until the press,
// and the press sends them — `approve_dispatch` takes the proposal as the
// person left it since 23.8 (#1641). These functions are the join between the
// draft's shapes, the controls' own and that body.

import type { CompleteChoice, ProposalCriterion, ProposalGateRow } from "@armada/components";
import type { ProposalLandingValue, WorkflowChoice } from "@armada/components";
import type {
  ApproveDispatch,
  GateChoice,
  JobDetail as JobWhole,
  LandingChoice,
  ManifestSummary,
  TierModels as WireTiers,
  WorkflowStep,
  WorkflowSummary,
} from "@armada/protocol";

import {
  criterionViewsOf,
  criterionWritten,
  criterionWrittenOf,
  decidedSaidOf,
  originSaidOf,
} from "./draft/criterion";
import type { CriterionView } from "./draft/criterion";
import type { JobDraft } from "./draft/held";
import { COMPLETE_WHEN_SERVED } from "./draft/landing";
import type { CompleteWhen, LandingRule } from "./draft/landing";
import { gateReadingOf, gatesForSteps, proposalViewOf, unmeantOf } from "./draft/proposal";
import type { GateView, ProposalView, RepositorySays, TierModels } from "./draft/proposal";
import { absoluteOf } from "./duration";

/**
 * Everything on the classifying screen a person may still move.
 *
 * **Held as the draft's own shapes, not the controls'.** These are what
 * `#1545` promoted onto the wire, so the screen's state is the thing sent at
 * the press — `approvalOf` is the one translation, rather than a second
 * vocabulary threaded through every control.
 */
export type ProposalEdits = {
  proposal: ProposalView;
  landing: LandingRule;
  criteria: readonly CriterionView[];
};

/**
 * What the screen opens on off a moment's draft, or `undefined` where the
 * moment carries no proposal. A real Job's is `proposalEditsOfWhole`.
 */
export function proposalEditsOf(draft: JobDraft | undefined): ProposalEdits | undefined {
  if (draft?.proposal === undefined) return undefined;
  return {
    proposal: draft.proposal,
    // A Job whose Manifest has not been read lands in nothing and says so,
    // rather than drawing a branch name nobody chose.
    landing: draft.landing ?? {
      target: null,
      from_ref: draft.proposal.from_ref,
      prs: "job",
      branching: "job",
      pr_mode: draft.proposal.pr_mode,
      complete_when: "delivered",
      land_together: [],
    },
    criteria: draft.criteria ?? [],
  };
}

/**
 * What a Job at its approval gate, or approved, is held to, as Fleet serves it
 * since 23.8 — the same shapes a moment's draft carries, so one screen draws
 * both and one press sends either.
 *
 * **A landing Fleet holds nothing for is the base, both ways**: absent
 * `target` and `from_ref` are the Manifest's base, which is not a branch name
 * to print — `landingValueOf` fills the field from the branch list instead.
 */
export function proposalEditsOfWhole(whole: JobWhole, machineCap: number | null): ProposalEdits {
  const proposal = proposalViewOf(whole, machineCap);
  return {
    proposal,
    landing: {
      target: whole.landing?.target ?? null,
      from_ref: proposal.from_ref,
      prs: "job",
      branching: "job",
      pr_mode: proposal.pr_mode,
      // Only `delivered` completes a Job, and the approval refuses the rest.
      complete_when: "delivered",
      land_together: [],
    },
    criteria: criterionViewsOf(whole),
  };
}

/**
 * The approval body: what a person moved since `before`, and nothing else —
 * `undefined` where nothing moved, which approves the proposal as it stands.
 *
 * **Only what moved, because a field left out is the proposal as it stands**
 * (`ApproveDispatch`). A gate is sent only where its boxes differ from what
 * its step declares — on a new workflow, what that workflow declares — so a
 * step nobody touched keeps the gate Fleet would have read anyway.
 *
 * **Except the Drone cap, which is sent whenever there is one**: Fleet reads a
 * cap left out as the machine's, so leaving out a cap the Job already holds
 * would take it off.
 *
 * **Criteria go whole, or not at all.** A line with an id is that line
 * reworded or not and keeps its origin; one without is new. A line left blank
 * is dropped rather than sent as a criterion with no words.
 */
export function approvalOf(
  edits: ProposalEdits,
  before: ProposalEdits,
  workflows: readonly WorkflowSummary[],
): ApproveDispatch | undefined {
  const { proposal, landing, criteria } = edits;
  const body: ApproveDispatch = {};
  if (proposal.title.trim() !== before.proposal.title.trim()) body.title = proposal.title.trim();
  if ((proposal.asked ?? "") !== (before.proposal.asked ?? "")) body.facts = proposal.asked ?? "";
  const moved = proposal.workflow_id !== before.proposal.workflow_id;
  if (moved) body.workflow_id = proposal.workflow_id;
  const declared = moved
    ? gatesForSteps(workflows.find((one) => one.id === proposal.workflow_id)?.steps ?? [])
    : before.proposal.gates;
  const gates = proposal.gates
    .filter((gate) => gateMoved(gate, declared.find((one) => one.step_id === gate.step_id)))
    .map(gateChoiceOf);
  if (gates.length > 0) body.gates = gates;
  if (criteriaMoved(criteria, before.criteria)) {
    body.criteria = criteria.filter((one) => one.text.trim() !== "").map(criterionWrittenOf);
  }
  if (TIERS.some((tier) => proposal.tiers[tier] !== before.proposal.tiers[tier])) {
    body.tiers = wireTiersOf(proposal.tiers);
  }
  if (proposal.drone_cap !== undefined) body.drone_cap = proposal.drone_cap;
  if (landingMoved(landing, before.landing)) body.landing = landingChoiceOf(landing);
  return Object.keys(body).length === 0 ? undefined : body;
}

/** The three tiers, in the map's own order. */
const TIERS = ["difficult", "medium", "easy"] as const;

/**
 * The cap taken off again. **Absent, never zero** — a `drone_cap` of nothing
 * is the machine's own cap holding, and `0` would read as a Job allowed no
 * Drone at all. The proposal screen and the approval panel both take it off.
 */
export function withoutCap(proposal: ProposalView): ProposalView {
  const { drone_cap: _dropped, ...rest } = proposal;
  return rest;
}

/** A tier left out is Armada picking: the draft's `null` is the wire's absent key. */
function wireTiersOf(tiers: TierModels): WireTiers {
  const wire: WireTiers = {};
  for (const tier of TIERS) {
    const model = tiers[tier];
    if (model !== null) wire[tier] = model;
  }
  return wire;
}

/** Whether a step's boxes differ from what it declares. A step nothing declares is a person's. */
function gateMoved(gate: GateView, declared: GateView | undefined): boolean {
  if (declared === undefined) return true;
  return (
    gate.checks !== declared.checks ||
    gate.judge !== declared.judge ||
    gate.you !== declared.you ||
    (gate.overridden === true) !== (declared.overridden === true)
  );
}

/** One step's gate as the approval sets it. `overridden` only where it is, which is Fleet's default. */
function gateChoiceOf(gate: GateView): GateChoice {
  return {
    step_id: gate.step_id,
    checks: gate.checks,
    judge: gate.judge,
    you: gate.you,
    ...(gate.overridden === true ? { overridden: true } : {}),
  };
}

/**
 * Whether the criteria a person left differ from what the Job holds — a line
 * added, taken off, reworded, or moved. Blank lines are compared as dropped,
 * since they are never sent.
 */
export function criteriaMoved(
  after: readonly CriterionView[],
  before: readonly CriterionView[],
): boolean {
  const kept = after.filter((one) => one.text.trim() !== "");
  return (
    kept.length !== before.length ||
    kept.some((one, at) => {
      const was = before[at];
      return (
        was === undefined ||
        one.criterion_id !== was.criterion_id ||
        one.text.trim() !== was.text.trim() ||
        one.verified_by !== was.verified_by
      );
    })
  );
}

function landingMoved(after: LandingRule, before: LandingRule): boolean {
  return (
    after.target !== before.target ||
    after.from_ref !== before.from_ref ||
    after.branching !== before.branching ||
    after.pr_mode !== before.pr_mode ||
    after.complete_when !== before.complete_when
  );
}

/** The landing as the approval sets it. A `null` ref is left out, which is the Manifest's base. */
function landingChoiceOf(landing: LandingRule): LandingChoice {
  return {
    ...(landing.target === null ? {} : { target: landing.target }),
    ...(landing.from_ref === null ? {} : { from_ref: landing.from_ref }),
    branching: landing.branching,
    pr_mode: landing.pr_mode,
    complete_when: landing.complete_when,
  };
}

/**
 * One row per step, in the order the workflow runs them.
 *
 * **The label comes off the frozen step and the gate off the draft.** A step
 * the draft has a gate for and Fleet has no record of reads by its id: the
 * gate is still this Job's answer, and dropping the row would draw a workflow
 * with a step missing.
 */
export function gateRowsOf(
  gates: readonly GateView[],
  whole: JobWhole | null,
  /** What the chosen workflow declares, where it is not the frozen one. */
  declared: ReadonlyMap<string, WorkflowStep> = new Map(),
  /** What this repository's policies say, for the rows that defer to one. */
  says: RepositorySays = {},
): ProposalGateRow[] {
  return gates.map((gate) => {
    // The frozen step first, then the chosen workflow's. A step neither holds
    // declares nothing, which is a Job naming a workflow Fleet has no record
    // of — and a box ticked on it says so.
    const step = whole?.steps.find((one) => one.step_id === gate.step_id) ??
      declared.get(gate.step_id);
    const reading = gateReadingOf(gate, says);
    // What the step declares, which a tick cannot change.
    const unmeant = unmeantOf(gate, {
      checks: (step?.checks?.length ?? 0) > 0,
      judge: (step?.judge_checks?.length ?? 0) > 0,
    });
    const label = step?.label ?? gate.step_id;
    const row: ProposalGateRow = {
      id: gate.step_id,
      label,
      // Fleet substitutes the id where a workflow declares no label, and an
      // id reads as one. `run.ts` and `stopped.ts` derive it the same way.
      labelIsAnIdentifier: label === gate.step_id,
      checks: gate.checks,
      judge: gate.judge,
      you: gate.you,
      advanceGate: reading.advance_gate,
      does: reading.does,
    };
    if (gate.repository_decides !== undefined) row.repositoryDecides = gate.repository_decides;
    if (gate.overridden !== undefined) row.overridden = gate.overridden;
    if (unmeant !== undefined) row.unmeant = unmeant;
    return row;
  });
}

/**
 * What this Job's repository says for each policy a gate can defer to.
 *
 * **Both words or neither**: a Fleet that sends these sends both, and one
 * older than 18.4 sends neither — so a row on an older Fleet reads as a
 * deference nothing resolved rather than as a policy nobody set.
 */
export function repositorySaysOf(manifest: ManifestSummary | undefined): RepositorySays {
  return {
    ...(manifest?.auto_merge === undefined ? {} : { auto_merge: manifest.auto_merge }),
    ...(manifest?.review_gate === undefined ? {} : { review_gate: manifest.review_gate }),
  };
}

/**
 * The workflows the picker offers, in the order Fleet holds them.
 *
 * **Only this Job's repository.** A workflow belongs to a Manifest, and a Job
 * cannot run one declared for another repository — offering them would be a
 * picker most of whose options are refused at the press.
 */
export function workflowChoicesOf(
  workflows: readonly WorkflowSummary[],
  manifestId: string,
): WorkflowChoice[] {
  return workflows
    .filter((workflow) => workflow.manifest_id === manifestId)
    .map((workflow) => ({ id: workflow.id, name: workflow.name, steps: workflow.steps.length }));
}

/**
 * This proposal on another workflow.
 *
 * **Every gate is rebuilt from the new workflow's steps.** A gate belongs to
 * a step, so the ticks a person moved name steps the new workflow may not
 * have; carrying them over by position would put a tick meant for `handoff`
 * on whatever runs fourth. A workflow Fleet holds no record of leaves the
 * gates alone — there are no steps to rebuild them from, and emptying the
 * list would draw a workflow with no steps at all.
 */
export function proposalOnWorkflow(
  proposal: ProposalView,
  workflows: readonly WorkflowSummary[],
  workflowId: string,
): ProposalView {
  const picked = workflows.find((workflow) => workflow.id === workflowId);
  if (picked === undefined) return { ...proposal, workflow_id: workflowId };
  return { ...proposal, workflow_id: workflowId, gates: gatesForSteps(picked.steps) };
}

/**
 * What each step of the chosen workflow declares, by its id.
 *
 * **Read off the workflow once a person has picked another**, and off the
 * Job's own frozen steps until then: `JobDetail.steps` is what the proposer
 * chose, and it holds no step of a workflow nobody approved.
 *
 * **The whole step, not its label.** It was the label alone for an hour, and
 * every row of a newly picked workflow then read *this step declares no
 * Check* under a ticked box — `unmeantOf` asks what the step declares, and a
 * step the Job never froze declared nothing it could see. A workflow's own
 * `checks` and `judge_checks` are exactly that answer.
 */
export function stepsDeclaredOf(
  workflows: readonly WorkflowSummary[],
  workflowId: string,
): ReadonlyMap<string, WorkflowStep> {
  const picked = workflows.find((workflow) => workflow.id === workflowId);
  return new Map((picked?.steps ?? []).map((step) => [step.step_id, step]));
}

/** One box moved on one step, with every other step carried through. */
export function gatesWith(
  gates: readonly GateView[],
  stepId: string,
  change: Partial<GateView>,
): GateView[] {
  return gates.map((gate) => (gate.step_id === stepId ? { ...gate, ...change } : gate));
}

/** What each answer to "complete when" is called, and whether Fleet can tell. */
const COMPLETE_LABEL: Readonly<Record<CompleteWhen, string>> = {
  pr_merged: "Its pull request merges",
  all_members_landed: "Every Job it holds has landed",
  pr_opened: "Its pull request is opened",
  delivered: "The step that delivers has delivered",
};

/**
 * The four answers, each said with whether anything on a Job's record answers
 * it today. **`COMPLETE_WHEN_SERVED` is read rather than restated**, so an
 * answer Fleet learns to observe stops being flagged here without this file
 * being touched.
 */
export function completeChoices(): CompleteChoice[] {
  return (Object.keys(COMPLETE_LABEL) as CompleteWhen[]).map((value) => ({
    value,
    label: COMPLETE_LABEL[value],
    served: COMPLETE_WHEN_SERVED[value],
  }));
}

/**
 * The landing rule as the controls hold it.
 *
 * **`null` is the base, drawn by its name where the branch list says which
 * branch that is** (#1605), and empty where nothing has said.
 */
export function landingValueOf(landing: LandingRule, base: string | null = null): ProposalLandingValue {
  return {
    target: landing.target ?? base ?? "",
    from: landing.from_ref ?? base ?? "",
    branching: landing.branching,
    completeWhen: landing.complete_when,
    prMode: landing.pr_mode,
  };
}

/**
 * The controls' answer, back on the rule.
 *
 * **An emptied field is `null` and not `""`.** `null` is the Manifest naming
 * no base, which is a state `amending.ts` already spells that way; an empty
 * string would be a branch with no name.
 *
 * **A field left on the base it was drawn with stays `null`**, so a press
 * that moved nothing sends nothing: `landingValueOf` drew the base's name
 * where the rule held none.
 */
export function landingWith(
  landing: LandingRule,
  moved: ProposalLandingValue,
  base: string | null = null,
): LandingRule {
  const refOf = (typed: string, was: string | null): string | null =>
    typed === "" || (was === null && typed === base) ? null : typed;
  return {
    ...landing,
    target: refOf(moved.target, landing.target),
    from_ref: refOf(moved.from, landing.from_ref),
    branching: moved.branching,
    complete_when: moved.completeWhen as CompleteWhen,
    pr_mode: moved.prMode,
  };
}

/** What the Job is held to, with where each line's words came from. */
export function criteriaRowsOf(criteria: readonly CriterionView[]): ProposalCriterion[] {
  return criteria.map((criterion, at) => {
    const from = originSaidOf(criterion);
    const row: ProposalCriterion = {
      id: criterion.criterion_id ?? String(at),
      text: criterion.text,
      ...(from === undefined ? {} : { origin: from.kind }),
      ...(from?.issue === undefined ? {} : { issue: from.issue }),
      decidedBy: decidedSaidOf(criterion),
    };
    // The instant is the issue's own edit and never Fleet's read — absent is
    // the ordinary case, where nothing has moved since.
    const moved =
      criterion.origin_moved_at === undefined
        ? null
        : absoluteOf(criterion.origin_moved_at);
    if (moved !== null) row.movedSince = moved;
    return row;
  });
}

/** One criterion reworded, with the rest carried through. */
export function criteriaWith(
  criteria: readonly CriterionView[],
  at: number,
  text: string,
): CriterionView[] {
  return criteria.map((criterion, index) =>
    index === at ? { ...criterion, text } : criterion,
  );
}

/**
 * One more line, appended empty.
 *
 * **At the foot, never at the head.** The order is the brief's and a citation
 * names a criterion's place in it (`concepts.tsx`, `#`), so a line inserted
 * above the others would renumber every citation already written.
 */
export function criteriaAdded(criteria: readonly CriterionView[]): CriterionView[] {
  return [...criteria, criterionWritten()];
}

/** One line taken off, with the rest carried through. */
export function criteriaWithout(
  criteria: readonly CriterionView[],
  at: number,
): CriterionView[] {
  return criteria.filter((_criterion, index) => index !== at);
}

/**
 * When the Job was approved, written out — or `undefined` where nobody has.
 *
 * **The instant is the whole of what separates the two moments**, so a
 * proposal carrying one whose timestamp cannot be read draws as frozen with no
 * date rather than as still editable.
 */
export function frozenAtOf(proposal: ProposalView): string | undefined {
  if (proposal.approved_at === undefined) return undefined;
  return absoluteOf(proposal.approved_at) ?? proposal.approved_at;
}
