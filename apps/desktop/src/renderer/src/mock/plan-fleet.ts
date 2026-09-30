// What a mock Fleet does with a person's own add or drop of a task — `#897`.
//
// **Both halves of what a board draws move together**: the Job's `work_plan`,
// which is what Fleet answers with, and the moment's draft groups, which is
// what an arc board draws while groups are not on the wire (`groupsOf` in
// `tab-plan-read.ts` takes the draft's first). One of them alone would leave
// the screen unchanged on one kind of scenario or the other.

import type { AddTask, DropTask, PlanTask, WorkPlan } from "@armada/protocol";
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

/** The groups with `drop.task` dropped, for the reason given. */
export function groupsDropping(groups: readonly GroupView[], drop: DropTask): GroupView[] {
  return groups.map((group) => ({
    ...group,
    tasks: group.tasks.map((task) =>
      task.id === drop.task ? { ...task, state: "dropped", reason: drop.reason.trim() } : task,
    ),
  }));
}
