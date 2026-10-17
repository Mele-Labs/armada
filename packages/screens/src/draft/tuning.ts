// What a person tunes per node on the approval canvas. Every field is on the
// wire: `StepTuning` since 23.20 and its `harness` since 23.23
// (`ApproveDispatch.tuning`), delivery since 23.24 (the local-only and auto-merge
// fields of `LandingChoice`, held on `LandingRule`), and the note back to the proposer since
// 23.25 (`to_proposer`). `approvalOf` sends them.
//
// Each default below is read off what Fleet already serves where it can be
// (`JobSummary.model`, a Judge's `panel_size`), and is otherwise Armada
// picking, which is `null`.

import type { DeclaredCheck, DeclaredJudge } from "@armada/protocol";

/** How hard a Drone thinks. `null` is Armada picking. */
export type Effort = "low" | "medium" | "high";

export const EFFORTS: readonly Effort[] = ["low", "medium", "high"];

/** One workflow step, tuned for this Job. */
export type StepTuning = {
  /** The model the step's Drone runs. `null` is the Job's own, or the tier's. */
  model: string | null;
  effort: Effort | null;
  /** The harness the step's Drone runs under, in its own name. `null` is the one Fleet runs by default (`ModelChoices.harnesses`'s first). */
  harness: string | null;
  /** Words handed to the Drone that picks the step up, beside the brief. Empty is none. */
  context: string;
  /** How many Judges answer each criterion. Read off `panel_size`, absent at one. */
  judges: number;
  /** Declared Checks this Job does not run, by name. */
  checks_off: readonly string[];
};

/**
 * How the work leaves the worktree, as the canvas's one control offers it.
 * `draft` and `ready` are `LandingRule.pr_mode`; `local` is the local-only field
 * — no pull request, no merge and no push: the work is held on its own branch.
 * **Read, never held**: `deliveryOf` derives it, so the two fields cannot
 * disagree with a third.
 */
export type Delivery = "local" | "draft" | "ready";

/** The one answer the control draws, from the two fields that hold it. */
export const deliveryOf = (local: boolean | undefined, prMode: "ready" | "draft"): Delivery => (local === true ? "local" : prMode);

/** Everything the canvas tunes per step. */
export type ApprovalTuning = {
  steps: Readonly<Record<string, StepTuning>>;
};

/** A Check's name as a step declares it: a Manifest Check by its name, the rest by kind. */
export const checkNameOf = (check: DeclaredCheck): string => check.name ?? check.kind;

/** A step's tuning before anybody moved it. */
export function stepTuningOf(judges: readonly DeclaredJudge[]): StepTuning {
  return {
    model: null,
    effort: null,
    harness: null,
    context: "",
    judges: judges[0]?.panel_size ?? 1,
    checks_off: [],
  };
}

/** The tuning a Job opens on. */
export function tuningOf(
  steps: readonly { step_id: string; judge_checks?: readonly DeclaredJudge[] }[],
): ApprovalTuning {
  return {
    steps: Object.fromEntries(steps.map((step) => [step.step_id, stepTuningOf(step.judge_checks ?? [])])),
  };
}

/** One step's tuning moved, the rest carried through. A step it holds nothing for starts from one Judge. */
export function tunedStep(tuning: ApprovalTuning, stepId: string, change: Partial<StepTuning>): ApprovalTuning {
  const was = tuning.steps[stepId] ?? stepTuningOf([]);
  return { ...tuning, steps: { ...tuning.steps, [stepId]: { ...was, ...change } } };
}

/** A Check turned on or off for one step. */
export function checksOffWith(off: readonly string[], name: string, runs: boolean): string[] {
  return runs ? off.filter((one) => one !== name) : [...off.filter((one) => one !== name), name];
}
