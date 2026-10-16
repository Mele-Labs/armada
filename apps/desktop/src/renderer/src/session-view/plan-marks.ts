// The plan as it reads with his marks on it. A mark is an edit or a note he has made and not yet sent: an edited title shows what it was,
// a removed task is struck, an added one is marked new, a note is pinned. Nothing here changes the plan; `viewOf` lays the marks over it,
// and `linesOf` words them for the message that goes to the agent.

import type { PlanMark, SuggestLine, WorkGroup, WorkPlan, WorkTask } from "@armada/screens/src/draft/sessions";

/** What the agent changed in the plan it recorded again, on one group or task. */
export type Since = { kind: "added" } | { kind: "renamed"; from: string };

export type ViewTask = WorkTask & {
  /** The group the task is marked under: where the plan has it, or where he added it. A mark on the task names this group. */
  home: string;
  /** What the text was before he renamed it. */
  was?: string;
  added?: true;
  removed?: true;
  /** The title of the group it came from. */
  movedFrom?: string;
  notes: readonly (PlanMark & { kind: "note" })[];
  since?: Since;
};

export type ViewGroup = Omit<WorkGroup, "tasks"> & {
  was?: string;
  agentWas?: string;
  added?: true;
  removed?: true;
  notes: readonly (PlanMark & { kind: "note" })[];
  tasks: readonly ViewTask[];
  /** Tasks the agent took out of the plan it recorded again, which the group no longer holds. */
  gone: readonly string[];
  since?: Since;
};

type Target = { group: string; task?: string };

const same = (mark: { group: string; task?: string }, target: Target): boolean => mark.group === target.group && mark.task === target.task;

export const hasMarks = (marks: readonly PlanMark[]): boolean => marks.length > 0;

/** The plan with his marks laid over it. */
export function viewOf(plan: WorkPlan, marks: readonly PlanMark[]): ViewGroup[] {
  const notes = (target: Target) => marks.filter((one): one is PlanMark & { kind: "note" } => one.kind === "note" && same(one, target));
  const renamed = (target: Target) => marks.find((one) => one.kind === "rename" && same(one, target));
  const removed = (target: Target) => marks.some((one) => one.kind === "remove" && same(one, target));
  const since = (id: string): Since | undefined => {
    if (plan.changes?.added.includes(id) === true) return { kind: "added" };
    const was = plan.changes?.renamed.find((one) => one.id === id);
    return was === undefined ? undefined : { kind: "renamed", from: was.from };
  };
  const moved = (task: string) => marks.find((one): one is PlanMark & { kind: "move" } => one.kind === "move" && one.task === task);
  const taskOf = (group: string, task: WorkTask): ViewTask => {
    const rename = renamed({ group, task: task.id });
    const target = { group, task: task.id };
    const fresh = since(task.id);
    return { ...task, home: group, ...(rename?.kind === "rename" ? { text: rename.to, was: rename.from } : {}), ...(removed(target) ? { removed: true as const } : {}), notes: notes(target), ...(fresh === undefined ? {} : { since: fresh }) };
  };
  const all = [...plan.groups, ...marks.flatMap((one) => (one.kind === "add" && one.task === undefined ? [{ id: one.id, agent: "general-purpose", title: one.text, paths: [], tasks: [] } satisfies WorkGroup] : []))];
  const titleOf = (id: string) => all.find((one) => one.id === id)?.title ?? id;
  return all.map((group): ViewGroup => {
    const rename = renamed({ group: group.id });
    const agent = marks.find((one) => one.kind === "agent" && one.group === group.id);
    const kept = group.tasks.filter((task) => moved(task.id) === undefined).map((task) => taskOf(group.id, task));
    const comes = plan.groups.flatMap((one) => one.tasks.map((task) => [one.id, task] as const)).flatMap(([from, task]) => {
      const move = moved(task.id);
      return move !== undefined && move.to === group.id ? [{ ...taskOf(from, task), movedFrom: titleOf(from) }] : [];
    });
    const added = marks.flatMap((one) => (one.kind === "add" && one.task !== undefined && one.group === group.id ? [{ id: one.id, home: group.id, text: one.text, state: "open" as const, added: true as const, notes: notes({ group: group.id, task: one.id }) }] : []));
    const fresh = since(group.id);
    return {
      ...group,
      ...(rename?.kind === "rename" ? { title: rename.to, was: rename.from } : {}),
      ...(agent?.kind === "agent" ? { agent: agent.to, agentWas: agent.from } : {}),
      ...(marks.some((one) => one.kind === "add" && one.task === undefined && one.id === group.id) ? { added: true as const } : {}),
      ...(removed({ group: group.id }) ? { removed: true as const } : {}),
      notes: notes({ group: group.id }),
      tasks: [...kept, ...comes, ...added],
      gone: plan.changes?.removed.filter((one) => one.group === group.id).map((one) => one.text) ?? [],
      ...(fresh === undefined ? {} : { since: fresh }),
    };
  });
}

/** Whether `target` is something he added, which a remove takes back instead of striking. */
const isAdded = (marks: readonly PlanMark[], target: Target): boolean => marks.some((one) => one.kind === "add" && (target.task === undefined ? one.task === undefined && one.id === target.group : one.id === target.task));

/** The text the target had before any rename of his. */
export const nameOf = (plan: WorkPlan, target: Target): string => {
  const group = plan.groups.find((one) => one.id === target.group);
  return target.task === undefined ? (group?.title ?? "") : (group?.tasks.find((one) => one.id === target.task)?.text ?? "");
};

/** Renames a target. A name put back, or an item he added, leaves no rename. */
export function rename(plan: WorkPlan, marks: readonly PlanMark[], target: Target, to: string): PlanMark[] {
  const text = to.trim();
  if (text === "") return [...marks];
  if (isAdded(marks, target)) return marks.map((one) => (one.kind === "add" && (target.task === undefined ? one.task === undefined && one.id === target.group : one.id === target.task) ? { ...one, text } : one));
  const from = nameOf(plan, target);
  const rest = marks.filter((one) => !(one.kind === "rename" && same(one, target)));
  return text === from ? rest : [...rest, { kind: "rename", ...target, from, to: text }];
}

/** Removes a target, or takes the removal back. Something he added is simply taken out, with what hung on it. */
export function remove(marks: readonly PlanMark[], target: Target): PlanMark[] {
  if (isAdded(marks, target)) {
    const id = target.task ?? target.group;
    return marks.filter((one) => !((one.kind === "add" && one.id === id) || ((one.kind === "note" || one.kind === "rename") && same(one, target)) || (target.task === undefined && one.kind === "add" && one.group === id)));
  }
  return marks.some((one) => one.kind === "remove" && same(one, target)) ? marks.filter((one) => !(one.kind === "remove" && same(one, target))) : [...marks, { kind: "remove", ...target }];
}

let made = 0;
/** An id for what he adds, which the agent does not know yet. */
export const newId = (prefix: string): string => `${prefix}-new-${(made += 1)}`;

export const addTask = (marks: readonly PlanMark[], group: string, text: string): PlanMark[] => {
  if (text.trim() === "") return [...marks];
  // `task` names the new task, which is what tells it from a group: an add with none is a group.
  const id = newId("t");
  return [...marks, { kind: "add", group, task: id, id, text: text.trim() }];
};
export const addGroup = (marks: readonly PlanMark[], title: string): PlanMark[] => {
  if (title.trim() === "") return [...marks];
  const id = newId("g");
  return [...marks, { kind: "add", group: id, id, text: title.trim() }];
};

/** Moves a task to another group; back where it was, it is not moved. */
export function move(plan: WorkPlan, marks: readonly PlanMark[], task: string, to: string): PlanMark[] {
  const home = plan.groups.find((one) => one.tasks.some((it) => it.id === task));
  if (home === undefined) return [...marks];
  const rest = marks.filter((one) => !(one.kind === "move" && one.task === task));
  return to === home.id ? rest : [...rest, { kind: "move", task, from: home.id, to }];
}

export function setAgent(plan: WorkPlan, marks: readonly PlanMark[], group: string, to: string): PlanMark[] {
  const from = plan.groups.find((one) => one.id === group)?.agent;
  const rest = marks.filter((one) => !(one.kind === "agent" && one.group === group));
  return from === undefined || from === to ? rest : [...rest, { kind: "agent", group, from, to }];
}

export const addNote = (marks: readonly PlanMark[], target: Target, text: string): PlanMark[] => (text.trim() === "" ? [...marks] : [...marks, { kind: "note", id: newId("n"), ...target, text: text.trim() }]);
export const dropNote = (marks: readonly PlanMark[], id: string): PlanMark[] => marks.filter((one) => !(one.kind === "note" && one.id === id));

/** His marks as the lines of the message the agent gets, each against its group or task. */
export function linesOf(plan: WorkPlan, marks: readonly PlanMark[]): SuggestLine[] {
  const groups = viewOf(plan, marks);
  const where = (target: Target): string => {
    const group = groups.find((one) => one.id === target.group);
    const title = group?.was ?? group?.title ?? "";
    if (target.task === undefined) return title;
    const task = group?.tasks.find((one) => one.id === target.task);
    return `${title} › ${task?.was ?? task?.text ?? ""}`;
  };
  const taskText = (id: string) => groups.flatMap((one) => one.tasks).find((one) => one.id === id)?.text ?? "";
  return marks.map((one): SuggestLine => {
    if (one.kind === "rename") return { verb: "Renamed", where: where(one), from: one.from, text: one.to };
    if (one.kind === "remove") return { verb: "Removed", where: where(one) };
    if (one.kind === "add") return { verb: "Added", where: one.task === undefined ? "Plan" : where({ group: one.group }), text: one.text };
    if (one.kind === "move") return { verb: "Moved", where: taskText(one.task), from: groups.find((g) => g.id === one.from)?.title ?? "", text: groups.find((g) => g.id === one.to)?.title ?? "" };
    if (one.kind === "agent") return { verb: "Subagent", where: where({ group: one.group }), from: one.from, text: one.to };
    return { verb: "Note", where: where(one), text: one.text };
  });
}

/** What the agent changed in the plan it recorded again, as the card lists it: what was added, renamed, and taken out. */
export function changeLines(plan: WorkPlan): { kind: "added" | "renamed" | "removed"; text: string; from?: string }[] {
  const changes = plan.changes;
  if (changes === undefined) return [];
  const text = (id: string) => plan.groups.flatMap((group) => [{ id: group.id, text: group.title }, ...group.tasks.map((task) => ({ id: task.id, text: task.text }))]).find((one) => one.id === id)?.text ?? id;
  return [
    ...changes.added.map((id) => ({ kind: "added" as const, text: text(id) })),
    ...changes.renamed.map((one) => ({ kind: "renamed" as const, text: text(one.id), from: one.from })),
    ...changes.removed.map((one) => ({ kind: "removed" as const, text: one.text })),
  ];
}
