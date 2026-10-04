// Which phase of a Job's run a workflow step belongs to: setting it up, the
// work, or delivering it. Draft, for `crates/ipc` (`WorkflowStep`), which
// carries no such field. The approval canvas lays its lanes by it (the owner,
// 4 Oct 2026); #1768's cross-workflow progress would read it too.
//
// Source of truth today: nothing. A step that declares none reads by what the
// wire already says — `delivers` is delivery, the rest the work — and never by
// its id or its place in the workflow.

export type StepPhase = "setup" | "work" | "delivery";

/** A step as a draft-reading fixture declares it: the wire's step, with its phase. */
export type PhasedStep = { phase?: StepPhase };

/** A step with its phase declared. A call rather than a literal, so a fixture typed by the wire still takes it. */
export function phased<T extends object>(step: T, phase: StepPhase): T & PhasedStep {
  return { ...step, phase };
}

/** A step's phase: its own, or what `delivers` says where it declares none. */
export function phaseOf(step: PhasedStep & { delivers?: boolean }): StepPhase {
  return step.phase ?? (step.delivers === true ? "delivery" : "work");
}
