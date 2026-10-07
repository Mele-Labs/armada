// Where a step may be added to one Job, and what hangs at each place. No React.
//
// **A place is a moment and a step**, a Trigger's (`docs/concepts/trigger.md`): before a step is its
// `step_starts`, after it is its `step_passes`, and `pr_opened` hangs from the delivering step.
// Whether a place is still ahead is Fleet's `core_model::behind`, read here the same way off the
// step rows so a `+` is not offered where Fleet would refuse it.

import type { AddedRuns, AddedStep, AddStep, JobDetail as JobWhole, TriggerMoment } from "@armada/protocol";
import { ordered } from "@armada/screens/src/facts";

export type Gap = { when: TriggerMoment; step: string };

export const sameGap = (a: Gap, b: Gap): boolean => a.when === b.when && a.step === b.step;

/** Before a Job is approved nothing runs, and an addition is held by the approval until the press. */
export function atGate(whole: JobWhole): boolean {
  return whole.job.status === "proposing" || whole.job.status === "awaiting_approval";
}

const OVER = new Set(["completed_failed", "completed_success", "killed", "rejected", "superseded"]);

/** The step the workflow sends the work out on. Without one `pr_opened` has nothing to hang from. */
export function delivererOf(whole: JobWhole): string | undefined {
  return ordered(whole).find((step) => step.delivers === true)?.step_id;
}

/**
 * Whether the moment is still to come. At the gate everything is. Once a Job runs, `step_starts`
 * and `pr_opened` are behind a step that has started, and `step_passes` behind one that has
 * advanced or that the Job has moved past.
 */
export function ahead(whole: JobWhole, gap: Gap): boolean {
  if (atGate(whole)) return true;
  if (OVER.has(whole.job.status)) return false;
  const steps = ordered(whole);
  const row = steps.find((step) => step.step_id === gap.step);
  if (row === undefined) return false;
  if (gap.when !== "step_passes") return row.state === "not_started";
  const current = steps.find((step) => step.step_id === whole.job.current_step_id);
  return row.state !== "advanced" && (current === undefined || row.ordinal >= current.ordinal);
}

/** What hangs at a place, in the order it was added. */
export const chainAt = (rows: readonly AddedStep[], gap: Gap): AddedStep[] => rows.filter((one) => sameGap(one, gap));

/**
 * The places along the run in the order their moments come, which is the order a canvas without
 * a Delivery lane draws them: a step's start, the pull request opening on the delivering step,
 * then the step passing.
 */
export function timelineOf(whole: JobWhole): Gap[] {
  const deliverer = delivererOf(whole);
  return ordered(whole).flatMap((step): Gap[] => [
    { when: "step_starts", step: step.step_id },
    ...(step.step_id === deliverer ? [{ when: "pr_opened" as const, step: step.step_id }] : []),
    { when: "step_passes", step: step.step_id },
  ]);
}

/** What the wire takes for one addition. The local id and the state are Bridge's own until Fleet answers. */
export function addStepOf(one: Pick<AddedStep, "runs" | "when" | "step" | "block" | "repair">): AddStep {
  return {
    runs: one.runs,
    when: one.when,
    step: one.step,
    ...(one.block ? { block: true } : {}),
    ...(one.repair ? { repair: true } : {}),
  };
}

export const NAMES_NOTHING = (runs: AddedRuns): boolean =>
  (runs.kind === "script" ? runs.command : runs.kind === "skill" ? runs.skill : runs.brief).trim() === "";

/** A step not yet added: what a `+` hands the panel that fills it in. */
export function blankAddition(gap: Gap, kind: AddedRuns["kind"]): AddedStep {
  return {
    id: "",
    runs: kind === "script" ? { kind, command: "" } : kind === "skill" ? { kind, skill: "" } : { kind, brief: "" },
    when: gap.when,
    step: gap.step,
    block: false,
    repair: false,
    placed: "approval",
    added_at: "",
    state: "pending",
  };
}

/** A step held at the gate, which has no id of Fleet's until the approval is pressed. */
export const heldId = (rows: readonly AddedStep[]): string => `draft-${rows.reduce((most, one) => Math.max(most, Number(one.id.replace("draft-", "")) || 0), 0) + 1}`;

/** The additions the approval holds as the person left them, against what it opened on. `undefined` is nothing moved. */
export function additionsOf(now: readonly AddedStep[], before: readonly AddedStep[]): AddStep[] | undefined {
  const sent = now.map(addStepOf);
  return JSON.stringify(sent) === JSON.stringify(before.map(addStepOf)) ? undefined : sent;
}
