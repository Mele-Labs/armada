// The Plan region's own read of a Job — pure, so the arithmetic is tested
// without a browser. `docs/concepts/plan.md`; the wire is `work-plan.ts`.

import type { JobDetail } from "@armada/protocol";
import type { TaskMarkState } from "@armada/components";

/** One task, as the Plan region draws it. `docs/concepts/plan.md`. */
export type PlanTaskRow = {
  id: string;
  title: string;
  state: TaskMarkState;
  /** Present on a dropped task, and on nothing else. */
  reason?: string;
  /** One line for what the other fields cannot hold. Absent where none. */
  note?: string;
  /** The paths the planner said this task touches. Absent where none. */
  scope?: readonly string[];
  /** What the planner said should prove it, and what the work said did. */
  expects?: string;
  shown?: string;
};

/**
 * The Job's plan, as `PlanWell` draws it. Absent draws nothing — a Job whose
 * workflow has no plan step, or one that has not reached it yet.
 */
export type PlanRegionData = {
  approach: string;
  /** Every task, dropped included, in plan order. */
  tasks: readonly PlanTaskRow[];
};

/**
 * The Plan region's read of a Job: a plan recorded, or the step that will
 * record one where none exists yet. `docs/concepts/plan.md`; `#1007`.
 *
 * **Absent draws no region at all** — a workflow that declares no step
 * recording a plan. `recorded: false` is the state between that and a full
 * `PlanRegionData`: the step is declared, and its own label is what the
 * placeholder names.
 */
export type PlanRegionRead =
  | ({ recorded: true } & PlanRegionData)
  | { recorded: false; stepLabel: string };

/** The six states a task's own wire string may be since 22.0. Anything else is `open`. */
const STATES: readonly TaskMarkState[] = ["open", "working", "handed_in", "done", "failed", "dropped"];

function markStateOf(state: string): TaskMarkState {
  return (STATES as readonly string[]).includes(state) ? (state as TaskMarkState) : "open";
}

/** The `DeclaredCheck.kind` `crates/core-model/src/job/workflow.rs` names `PLAN_RECORDED`. */
const PLAN_RECORDED = "plan_recorded";

/**
 * The Plan region's props, or `undefined` for a Job whose workflow declares
 * no step recording a plan — nothing is drawn empty on this screen.
 *
 * **Read off the declared checks, never `workflow_id`.** A step's own
 * `checks` say whether it records the plan; the workflow's name does not,
 * and `#1006` lets any step declare `plan_recorded` rather than only the one
 * a workflow happens to call `plan`.
 */
export function planOf(whole: JobDetail | null): PlanRegionRead | undefined {
  if (whole === null) return undefined;
  const plan = whole.work_plan;
  if (plan !== undefined) {
    const tasks: PlanTaskRow[] = plan.tasks.map((task) => ({
      id: task.id,
      title: task.title,
      state: markStateOf(task.state),
      reason: task.reason,
      note: task.note,
      scope: task.scope,
      expects: task.expects,
      shown: task.shown,
    }));
    return { recorded: true, approach: plan.approach, tasks };
  }
  const pending = whole.steps.find((step) =>
    (step.checks ?? []).some((check) => check.kind === PLAN_RECORDED),
  );
  return pending === undefined ? undefined : { recorded: false, stepLabel: pending.label };
}
