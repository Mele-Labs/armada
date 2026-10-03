// What a mock Fleet does with a person's own acts on a plan: add and drop
// (`#897`), since 23.4 restart (`#1656`) and move (`#1685`), and since 23.6
// Edit this task (`#1657`).
//
// **Both halves of what a board draws move together**: the Job's `work_plan`,
// which is what Fleet answers with, and the moment's draft groups, which is
// what an arc board draws (`groupsOf` in `tab-plan-read.ts` takes the draft's
// first). One of them alone would leave the screen unchanged on one kind of
// scenario or the other.

import type { AddTask, DropTask, EditTask, MovePlan, PlanTask, StepDetail, WorkPlan } from "@armada/protocol";
import type { GroupView } from "@armada/screens/src/draft/group";
import type { TaskView } from "@armada/screens/src/draft/task";

/** The next `T<n>` neither the plan nor the groups hold. Ids are never reused. */
export function nextTaskId(plan: WorkPlan, groups: readonly GroupView[] = []): string {
  const ids = [...plan.tasks, ...groups.flatMap((group) => group.tasks)].map((task) => task.id);
  const numbers = ids.map((id) => Number(/^T(\d+)$/.exec(id)?.[1] ?? 0));
  return `T${Math.max(0, ...numbers) + 1}`;
}

/** The plan with a new open task after `add.after`, or at the end where that is `""`. */
export function planAdding(plan: WorkPlan, id: string, add: AddTask): WorkPlan {
  const task: PlanTask = { id, title: add.title.trim(), state: "open" };
  if (add.note.trim() !== "") task.note = add.note.trim();
  if (add.scope.length > 0) task.scope = [...add.scope];
  if (add.expects.trim() !== "") task.expects = add.expects.trim();
  const at = add.after === "" ? plan.tasks.length : plan.tasks.findIndex((one) => one.id === add.after) + 1;
  return { ...plan, tasks: [...plan.tasks.slice(0, at), task, ...plan.tasks.slice(at)] };
}

/** The plan with `drop.task` dropped, for the reason given. */
export function planDropping(plan: WorkPlan, drop: DropTask): WorkPlan {
  return {
    ...plan,
    tasks: plan.tasks.map((task) =>
      task.id === drop.task ? { ...task, state: "dropped", reason: drop.reason.trim() } : task,
    ),
  };
}

/**
 * The groups with the new task in the group holding `add.after` — the last
 * group where that is `""` — run the way the task before it is run.
 */
export function groupsAdding(groups: readonly GroupView[], id: string, add: AddTask): GroupView[] {
  const tasks = groups.flatMap((group) => group.tasks);
  const before = add.after === "" ? tasks.at(-1) : tasks.find((task) => task.id === add.after);
  if (before === undefined) return [...groups];
  const task: TaskView = {
    id,
    title: add.title.trim(),
    scope: [...add.scope],
    state: "open",
    touched_after_done: false,
    group: before.group,
    concurrent_with: [],
    tier: before.tier,
    model: before.model,
    treatment: before.treatment,
    cases: [],
    coord: { ...before.coord, task: id },
  };
  if (add.note.trim() !== "") task.note = add.note.trim();
  if (add.expects.trim() !== "") task.expects = add.expects.trim();
  return groups.map((group) => {
    const at = group.tasks.findIndex((one) => one.id === before.id);
    if (at === -1) return group;
    return {
      ...group,
      tasks: [...group.tasks.slice(0, at + 1), task, ...group.tasks.slice(at + 1)],
      scope: [...new Set([...group.scope, ...add.scope])],
    };
  });
}

/**
 * Restart this task (#1656): the failed task reopened with its Drone on it,
 * which is where a real Fleet has it a turn later. `undefined` where the task
 * has not failed, as Fleet refuses one — in the plan, or in the draft groups an
 * arc moment draws instead (`failedInDraft`) — unless it is a done task in a
 * group the Judge refused, which Fleet restarts too (owner, 2 Oct 2026).
 */
export function planRestarting(
  plan: WorkPlan,
  taskId: string,
  failedInDraft = false,
  steps: readonly StepDetail[] = [],
): WorkPlan | undefined {
  const state = plan.tasks.find((task) => task.id === taskId)?.state;
  if (!failedInDraft && state !== "failed" && !(state === "done" && judgeRefused(plan, steps, taskId))) return undefined;
  return {
    ...plan,
    tasks: plan.tasks.map((task) => {
      if (task.id !== taskId) return task;
      const { failed_reason: _failed, ...rest } = task;
      return { ...rest, state: "working" };
    }),
  };
}

/**
 * Fleet's reading: the task's group last ended `gate_failure` with no task in
 * it failed, and the step that run was filed under stopped. A question holds
 * that step at `awaiting_human` instead, and is refused.
 */
function judgeRefused(plan: WorkPlan, steps: readonly StepDetail[], taskId: string): boolean {
  const group = plan.groups?.find((one) => one.tasks.includes(taskId));
  const last = group?.attempts?.at(-1);
  return (
    group !== undefined &&
    last?.ended_at !== undefined &&
    last.verdict?.trigger === "gate_failure" &&
    steps.find((step) => step.step_id === last.step_id)?.state === "stopped" &&
    !plan.tasks.some((task) => group.tasks.includes(task.id) && task.state === "failed")
  );
}

/** The groups with the restarted task working, as `planRestarting` has it. */
export function groupsRestarting(groups: readonly GroupView[], taskId: string): GroupView[] {
  return groups.map((group) => ({
    ...group,
    tasks: group.tasks.map((task) => {
      if (task.id !== taskId) return task;
      const { failed_reason: _failed, ...rest } = task;
      return { ...rest, state: "working" };
    }),
  }));
}

/**
 * A move (#1685), by `after` as Fleet takes one: a task into `move.group` after
 * the task named or first in it, or a group after the group named or first.
 */
export function planMoving(plan: WorkPlan, move: MovePlan): WorkPlan {
  if (move.task === undefined) {
    const groups = [...(plan.groups ?? [])];
    const from = groups.findIndex((group) => group.id === move.group);
    if (from < 0) return plan;
    const [moved] = groups.splice(from, 1);
    const at = move.after === undefined ? 0 : groups.findIndex((group) => group.id === move.after) + 1;
    groups.splice(at, 0, moved!);
    const order = groups.flatMap((group) => group.tasks);
    return {
      ...plan,
      groups,
      tasks: [...plan.tasks].sort((a, b) => order.indexOf(a.id) - order.indexOf(b.id)),
    };
  }
  const task = plan.tasks.find((one) => one.id === move.task);
  if (task === undefined) return plan;
  const rest = plan.tasks.filter((one) => one.id !== move.task);
  const placed = { ...task, group: move.group };
  const at =
    move.after === undefined
      ? Math.max(0, rest.findIndex((one) => one.group === move.group))
      : rest.findIndex((one) => one.id === move.after) + 1;
  const tasks = [...rest.slice(0, at), placed, ...rest.slice(at)];
  return {
    ...plan,
    tasks,
    ...(plan.groups === undefined
      ? {}
      : {
          groups: plan.groups.map((group) => ({
            ...group,
            tasks: tasks.filter((one) => one.group === group.id).map((one) => one.id),
          })),
        }),
  };
}

/** The groups with one move applied, by `after`. **Ordinals stay the plan's.** */
export function groupsMoving(groups: readonly GroupView[], move: MovePlan): GroupView[] {
  if (move.task === undefined) {
    const from = groups.findIndex((group) => group.id === move.group);
    if (from < 0) return [...groups];
    const rest = groups.filter((group) => group.id !== move.group);
    const at = move.after === undefined ? 0 : rest.findIndex((group) => group.id === move.after) + 1;
    rest.splice(at, 0, groups[from]!);
    return rest;
  }
  const task = groups.flatMap((group) => group.tasks).find((one) => one.id === move.task);
  if (task === undefined) return [...groups];
  return groups.map((group) => {
    const tasks = group.tasks.filter((one) => one.id !== move.task);
    if (group.id === move.group) {
      const at = move.after === undefined ? 0 : tasks.findIndex((one) => one.id === move.after) + 1;
      tasks.splice(at, 0, { ...task, group: group.id });
    }
    return { ...group, tasks };
  });
}

/** Fleet's rule for an edit: an open or a failed task, and nothing else. */
const EDITABLE: readonly string[] = ["open", "failed"];

/**
 * Edit this task (#1657), as Fleet keeps one: the fields sent, trimmed, over
 * the task's own, and `undefined` where the task is not open or failed — in
 * the plan, or in the draft groups an arc moment draws instead.
 */
export function planEditing(plan: WorkPlan, taskId: string, edit: EditTask, editableInDraft = false): WorkPlan | undefined {
  const was = plan.tasks.find((task) => task.id === taskId);
  if (!editableInDraft && (was === undefined || !EDITABLE.includes(was.state))) return undefined;
  return { ...plan, tasks: plan.tasks.map((task) => (task.id === taskId ? edited(task, edit) : task)) };
}

/** The groups with the same edit on the same task, as `planEditing` has it. */
export function groupsEditing(groups: readonly GroupView[], taskId: string, edit: EditTask): GroupView[] {
  return groups.map((group) => ({
    ...group,
    tasks: group.tasks.map((task) => (task.id === taskId ? edited(task, edit) : task)),
  }));
}

/** One task with an edit's fields over its own. A field emptied is left out, as Fleet serves it. */
function edited<T extends { title: string; note?: string; scope?: string[]; expects?: string; model?: string }>(
  task: T,
  edit: EditTask,
): T {
  const next: T = { ...task };
  if (edit.title !== undefined) next.title = edit.title.trim();
  if (edit.scope !== undefined) next.scope = edit.scope.map((path) => path.trim()).filter((path) => path !== "");
  if (edit.model !== undefined) next.model = edit.model;
  for (const key of ["note", "expects"] as const) {
    const sent = edit[key];
    if (sent === undefined) continue;
    if (sent.trim() === "") delete next[key];
    else next[key] = sent.trim();
  }
  return next;
}

/** The groups with `drop.task` dropped, for the reason given. */
export function groupsDropping(groups: readonly GroupView[], drop: DropTask): GroupView[] {
  return groups.map((group) => ({
    ...group,
    tasks: group.tasks.map((task) =>
      task.id === drop.task ? { ...task, state: "dropped", reason: drop.reason.trim() } : task,
    ),
  }));
}
