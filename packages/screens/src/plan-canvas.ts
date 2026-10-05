// The plan as a graph — its groups, and the tasks inside each one. The Plan
// tab's Graph view, beside the List it already had. `#1539`.
//
// **It came off the Workflow canvas whole** (owner, 25 Sep 2026): the layout,
// the cards and the placement are the ones that used to hang under the step
// that recorded the plan. What was left behind is the step row above them, and
// with it the second edge — there are no step nodes here, so nothing can draw
// *and this step worked it*. That cost was stated and taken; a Workflow step's
// panel carries a card of the plan instead, and pressing it lands here.
//
// **Placement is computed here, from plan order.** The canvas holds none, so
// the numbers below are the layout and they are unit-tested in this package.

import {
  GROUP_STATE,
  TASK_STATE,
  type WorkflowCanvasEdge,
  type WorkflowCanvasNode,
  type WorkflowStepCardProps,
  type StepActivity,
} from "@armada/components";

import type { GroupState, GroupView } from "./draft/group";
import { SHELL_UNSEEN } from "./plan-board";
import { awaitingSaid } from "./tab-plan-read";
import type { TaskState, TaskView } from "./draft/task";

/**
 * The layout, in the canvas's own coordinates.
 *
 * `TASK_ACROSS` is `--w-workflow-group-node` (228) plus an 88 gap: room for a
 * smooth step to turn twice, clearing each card by the canvas's 20, and still
 * run straight between. At 28 the turns could not fit and each edge hooked
 * back on itself (owner, 29 Sep 2026). `TASK_APART` is one task's pitch down
 * that column and `AFTER_GROUP` the gap to the group below. Numbers rather
 * than tokens because React Flow places by number and a `var()` cannot reach
 * it.
 */
const TASK_ACROSS = 316;
const TASK_APART = 104;
const AFTER_GROUP = 24;
/**
 * What a task carrying the awaiting line adds to its pitch. The line wraps
 * rather than clips ("Submitted · awaiting checks" is the fact), so the node
 * is taller by the wrapped lines: `--leading-xs` (20) each, and the sentence
 * takes up to two beyond the one a task's facts already draw — 40, so nothing
 * below overlaps. React Flow places by number, so it is a number here.
 */
const LINE_EXTRA = 40;
/** A group holding no task still takes a row of its own. */
const GROUP_APART = 104;

/** `group:` and `task:`, so a node id is never mistaken for another kind in a join. */
export const groupNodeId = (groupId: string): string => `group:${groupId}`;
export const taskNodeId = (taskId: string): string => `task:${taskId}`;

/** The task a node id names, or nothing where it names a group. */
export function taskOfNodeId(nodeId: string): string | undefined {
  return nodeId.startsWith("task:") ? nodeId.slice("task:".length) : undefined;
}

/** `1 group`, `4 groups`, `2 criteria` — the plural is given where it is not the `s`. */
export function plural(count: number, one: string, many = `${one}s`): string {
  return `${count} ${count === 1 ? one : many}`;
}

/**
 * The group states that are still working, so the card sweeps. **The sweep is
 * all a group's activity decides**: its glyph, hue and word are `GROUP_STATE`'s
 * row, the one the list's group head reads, handed to the card as `mark`.
 */
const GROUP_WORKING: ReadonlySet<GroupState> = new Set(["running", "joining", "checking"]);

/**
 * A task's state mapped the same way. `dropped` takes `stopped`, the step
 * machine's word for work that ended without advancing; the task's own word is
 * printed beside the mark either way.
 */
const TASK_ACTIVITY: Record<TaskState, StepActivity> = {
  open: "not_started",
  working: "running",
  // Its agent has stopped, so nothing sweeps; `taskCard` draws its own mark.
  handed_in: "not_started",
  done: "advanced",
  failed: "failed",
  dropped: "stopped",
};

/** One group's card. Its registry row's glyph, hue and verb — what the list says. */
function groupCard(group: GroupView, onOpen: (() => void) | undefined): WorkflowStepCardProps {
  const row = GROUP_STATE[group.state];
  const facts: { value: string; hint?: string }[] = [{ value: plural(group.tasks.length, "task") }];
  if (group.checks_selected.length > 0) facts.push({ value: plural(group.checks_selected.length, "check") });
  if (group.concurrent) facts.push({ value: "at the same time", hint: SHELL_UNSEEN });
  if (group.retry_count > 0) facts.push({ value: `run again ${plural(group.retry_count, "time")}` });
  return {
    kind: "group",
    name: `Group ${group.ordinal}`,
    activity: GROUP_WORKING.has(group.state) ? "running" : "not_started",
    ...(row?.icon && row.statusToken ? { mark: { icon: row.icon, token: row.statusToken } } : {}),
    said: row?.verb ?? group.state,
    facts,
    ...(onOpen === undefined ? {} : { onOpen }),
  };
}

/**
 * One task's card. **The id is the first fact and never the name** — `T5`
 * alone would be a graph of identifiers.
 *
 * **Two facts, because a third wraps and a wrapped card overlaps the one under
 * it.** Turns displace the file count once something has run: the scope is
 * what there is to say before, and what it has taken is what there is to say
 * after.
 */
export function taskCard(task: TaskView, onOpen: (() => void) | undefined): WorkflowStepCardProps {
  const facts = [{ value: task.id }];
  if (task.turns !== undefined) facts.push({ value: plural(task.turns, "turn") });
  else if (task.scope.length > 0) facts.push({ value: plural(task.scope.length, "file") });
  // **A handed-in task is not a working one** (owner, 5 Oct 2026): its own
  // glyph and hue from the registry, and the sentence under its name.
  const awaiting = awaitingSaid(task.state);
  const row = TASK_STATE[task.state];
  return {
    kind: "task",
    name: task.title,
    activity: TASK_ACTIVITY[task.state],
    ...(awaiting !== undefined && row?.icon && row.statusToken
      ? { mark: { icon: row.icon, token: row.statusToken }, line: awaiting }
      : {}),
    said: awaiting ?? row?.verb ?? task.state,
    facts,
    ...(onOpen === undefined ? {} : { onOpen }),
  };
}

export type PlanGraphReading = {
  /** The plan's groups, in the order they run. Empty draws nothing. */
  groups: readonly GroupView[];
  /**
   * Opens a task in the sheet the list opens it in — **one destination for one
   * task**, so the toggle changes the arrangement and never what a press does.
   * Absent draws cards that are not controls.
   */
  onOpenTask?: (taskId: string) => void;
  /** The task a person has open, so the card it came from says which one it is. */
  openTask?: string | null;
  /**
   * Opens a group in its own panel — its tasks, Checks and Tests, with Add
   * task in the head (owner, 30 Sep 2026). Absent draws group cards that are
   * not controls.
   */
  onOpenGroup?: (groupId: string) => void;
  /** The group a person has open, drawn selected the way an open task is. */
  openGroup?: string | null;
};

export type PlanGraph = {
  nodes: WorkflowCanvasNode[];
  edges: WorkflowCanvasEdge[];
  /**
   * What the canvas opens on when the whole plan will not read: the groups
   * alone. A task is left out — it is the finest grain on the graph, and what
   * a person pans or presses to once they have found its group.
   */
  opensOn: string[][];
};

/**
 * The plan, placed: a column of groups, each with its own tasks beside it.
 *
 * **A group is a root here.** On Workflow a group hung off the step that wrote
 * it; on this tab there is no step to hang from, so the plan is as many small
 * trees as it has groups.
 */
export function planGraphOf({ groups, onOpenTask, openTask, onOpenGroup, openGroup }: PlanGraphReading): PlanGraph {
  const nodes: WorkflowCanvasNode[] = [];
  const edges: WorkflowCanvasEdge[] = [];

  let down = 0;
  for (const group of groups) {
    const groupId = groupNodeId(group.id);
    const card = groupCard(group, onOpenGroup === undefined ? undefined : () => onOpenGroup(group.id));
    nodes.push({
      id: groupId,
      position: { x: 0, y: down },
      card: group.id === openGroup ? { ...card, selected: true } : card,
    });

    let under = down;
    group.tasks.forEach((task) => {
      const open = onOpenTask === undefined ? undefined : () => onOpenTask(task.id);
      const card = taskCard(task, open);
      nodes.push({
        id: taskNodeId(task.id),
        position: { x: TASK_ACROSS, y: under },
        card: task.id === openTask ? { ...card, selected: true } : card,
      });
      under += TASK_APART + (card.line === undefined ? 0 : LINE_EXTRA);
      edges.push({
        id: `${groupId}>${taskNodeId(task.id)}`,
        source: groupId,
        target: taskNodeId(task.id),
        kind: "holds",
      });
    });
    down = Math.max(down + GROUP_APART, under + AFTER_GROUP);
  }

  return { nodes, edges, opensOn: [groups.map((group) => groupNodeId(group.id))] };
}
