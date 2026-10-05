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
  type WorkflowStepNeed,
  type StepActivity,
} from "@armada/components";

import type { GroupState, GroupView } from "./draft/group";
import { SHELL_UNSEEN } from "./plan-board";
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
/** The row a need adds under a card's name, as `workflow-canvas.ts`'s `ROW`. */
const NEED_ROW = 28;

/** What a Drone held on a command shows on its task, and on the group holding it. */
const HELD_NEED: WorkflowStepNeed = { says: "Command to allow", tone: "waiting" };
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
  // In flight until its Checks answer, `group_state.checking`'s reading.
  handed_in: "running",
  done: "advanced",
  failed: "failed",
  dropped: "stopped",
};

/** One group's card. Its registry row's glyph, hue and verb — what the list says. */
function groupCard(
  group: GroupView,
  onOpen: (() => void) | undefined,
  held = false,
): WorkflowStepCardProps {
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
    ...(held ? { needs: [HELD_NEED] } : {}),
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
export function taskCard(
  task: TaskView,
  onOpen: (() => void) | undefined,
  held = false,
): WorkflowStepCardProps {
  const facts = [{ value: task.id }];
  if (task.turns !== undefined) facts.push({ value: plural(task.turns, "turn") });
  else if (task.scope.length > 0) facts.push({ value: plural(task.scope.length, "file") });
  return {
    kind: "task",
    name: task.title,
    activity: TASK_ACTIVITY[task.state],
    said: TASK_STATE[task.state]?.verb ?? task.state,
    facts,
    ...(held ? { needs: [HELD_NEED] } : {}),
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
  /** The task whose Drone is held on a command, where the wire can say which. Its card and its group's ask. */
  heldTask?: string;
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
export function planGraphOf({
  groups,
  onOpenTask,
  openTask,
  onOpenGroup,
  openGroup,
  heldTask,
}: PlanGraphReading): PlanGraph {
  const nodes: WorkflowCanvasNode[] = [];
  const edges: WorkflowCanvasEdge[] = [];

  let down = 0;
  for (const group of groups) {
    const groupId = groupNodeId(group.id);
    const holds = heldTask !== undefined && group.tasks.some((task) => task.id === heldTask);
    const card = groupCard(group, onOpenGroup === undefined ? undefined : () => onOpenGroup(group.id), holds);
    nodes.push({
      id: groupId,
      position: { x: 0, y: down },
      card: group.id === openGroup ? { ...card, selected: true } : card,
    });

    // The held task's card draws a row more, so what is under it moves down.
    let column = 0;
    group.tasks.forEach((task) => {
      const open = onOpenTask === undefined ? undefined : () => onOpenTask(task.id);
      const isHeld = task.id === heldTask;
      const card = taskCard(task, open, isHeld);
      nodes.push({
        id: taskNodeId(task.id),
        position: { x: TASK_ACROSS, y: down + column },
        card: task.id === openTask ? { ...card, selected: true } : card,
      });
      edges.push({
        id: `${groupId}>${taskNodeId(task.id)}`,
        source: groupId,
        target: taskNodeId(task.id),
        kind: "holds",
      });
      column += TASK_APART + (isHeld ? NEED_ROW : 0);
    });
    down += Math.max(GROUP_APART, column + AFTER_GROUP);
  }

  return { nodes, edges, opensOn: [groups.map((group) => groupNodeId(group.id))] };
}
