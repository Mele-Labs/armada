import { useState, type DragEvent, type KeyboardEvent } from "react";
import { ChevronRight, TriangleAlert } from "lucide-react";
import { GROUP_STATE } from "../../generated/vocabulary";
import { Button } from "../../primitives/Button/Button";
import { Card, CardContent, CardHeader, CardTitle } from "../../primitives/Card/Card";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { keyFor } from "../../actions";
import { Clamped } from "../Clamped/Clamped";
import { GroupBoundary, type GroupBoundaryProps } from "../GroupBoundary/GroupBoundary";
import { GroupShape } from "../GroupShape/GroupShape";
import { GuideMark } from "../GuideMark/GuideMark";
import { GUIDE_GROUP_ORDER, GUIDE_PLAN_ASKS } from "../../guides";
import { TaskMark, type TaskMarkState } from "../TaskMark/TaskMark";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { PlanDropForm } from "./PlanDropForm";

/**
 * Plan board — a plan read as the groups it will run in, one card each.
 *
 * **The card is the group, because the group is where the Checks run**
 * (`#1530`, 21 Sep). A task carries what its own agent does; the boundary
 * under it carries what runs once every task in the card has stopped.
 *
 * **A group says where it writes, not every file it writes.** Fifteen paths
 * under a header said neither where the gate measures nor where the next group
 * collides — the owner asked what they were for (28 Sep 2026). The root says
 * the first; an overlap is its own warning inside the group that has one.
 *
 * **It composes no sentence** — `packages/screens/src/tab-plan-read.ts`.
 */

/** Where a group is, for its hue alone. The word is `says`, which is prose. */
export type PlanGroupState =
  | "pending"
  | "running"
  | "joining"
  | "checking"
  | "passed"
  | "failed"
  | "retrying"
  | "landed";

/** One task's row. `mark` is the whole of its state — no row prints a word. */
export type PlanBoardTask = {
  /** `T1`, `T2`, … as the plan numbered it. */
  id: string;
  title: string;
  mark: TaskMarkState;
  /** The model the planner's tier resolved to. */
  model: string;
  /** How hard the planner thought it was. Read in the inspector, not the row. */
  tier: string;
  /** `its own agent` — how it is run. Absent before the plan says. */
  runBy?: string;
  /** `beside T5`, written by the caller. Absent where it runs alone. */
  besideSays?: string;
  /** `touched later · T7` — a later task edited a file this one had finished. */
  touchedSays?: string;
  /** Why its agent stopped. Present on a failed task and on nothing else. */
  failedReason?: string;
  /** `34 turns`, or nothing before its agent started. */
  turnsSays?: string;
  /** `~$2.40`. Absent until its own agent stopped — a live cost is invented. */
  costSays?: string;
};

/**
 * One case at a boundary or on a task. **`reads` is the whole claim**: a case
 * with no spec reads `not covered` and never green.
 */
export type PlanBoardTest = {
  id: string;
  /** The spec's repository path, or the case's own id where it has no spec. */
  spec: string;
  reads: "owed" | "not covered" | "dropped";
  /** What dropped it. Present on `dropped` and on nothing else. */
  droppedSays?: string;
};

/**
 * Where this group writes, as one value. **No count beside it** (owner, 30 Sep
 * 2026): how many paths the root stopped naming was a number nobody read.
 */
export type PlanBoardScope = { root: string };

/**
 * A file this group claims that another group claims too. **Inside the group**
 * — the owner's call of 28 Sep 2026: a warning about group 4 belongs on
 * group 4's card, not in a band above the whole plan.
 */
export type PlanBoardOverlap = {
  /** `Group 3 writes these files too`, the caller's own sentence. */
  says: string;
  paths: readonly string[];
};

/**
 * One overlap warning, in the caution treatment — a group's card, and Plan's
 * task panel under its Files, where a later task edited a file this one had
 * already finished. **One drawing for both**, so the two read as the same
 * kind of warning. No `paths` draws the sentence alone.
 */
export function PlanOverlap({ says, paths = [] }: { says: string; paths?: readonly string[] }) {
  return (
    <div className="armada-plan-board__overlap" role="note">
      <TriangleAlert size={12} strokeWidth={2} aria-hidden />
      <span className="armada-plan-board__overlap-says">{says}</span>
      {paths.length === 0 ? null : (
        <ul className="armada-plan-board__overlap-paths">
          {paths.map((path) => (
            <li key={path}>{path}</li>
          ))}
        </ul>
      )}
    </div>
  );
}

/**
 * Where a group is: `GROUP_STATE`'s glyph and hue beside its word — the list's
 * head and the group's panel both draw this, so the two cannot disagree.
 * Plain text rather than a pill, as before; the hue is the registry's own
 * token, read the way `Badge` reads its stem, so this file never decides
 * which states are bad.
 */
export function PlanGroupStateWord({ state, says }: { state: PlanGroupState; says: string }) {
  const rendering = GROUP_STATE[state];
  const Icon = rendering?.icon;
  return (
    <span
      className="armada-plan-board__state"
      data-state={state}
      style={rendering?.statusToken ? { color: `var(${rendering.statusToken})` } : undefined}
    >
      {Icon ? <Icon size={12} strokeWidth={2} aria-hidden /> : null}
      {says}
    </span>
  );
}

export type PlanBoardGroup = {
  id: string;
  /** Its position in the step, counted from one. */
  ordinal: number;
  state: PlanGroupState;
  /** Where the group is, in words — `GROUP_STATE`'s verb: `running`, `passed`, `retrying`. */
  says: string;
  /**
   * Whether the tasks run at the same time. **Drawn, not named** — `GroupShape`,
   * the owner's sketch — because `2 tasks, one after another` was a label for
   * something a reader pictures anyway (owner, 30 Sep 2026).
   */
  concurrent: boolean;
  /** What the drawing says to somebody who cannot see it — `2 tasks, one after another`. */
  shapeSays: string;
  scope: PlanBoardScope;
  tasks: readonly PlanBoardTask[];
  /** Every other group that claims a file this one does. Empty draws nothing. */
  overlaps?: readonly PlanBoardOverlap[];
  /**
   * What runs once every task in the card has stopped, and what it came to —
   * **`GroupBoundary`'s own reading**, so the commit a group left and the
   * failed Check's own record are drawn once rather than spelled twice.
   */
  boundary: GroupBoundaryProps;
};

/**
 * Where a person dropped a group or a task (owner, 30 Sep 2026: *edits to the
 * plan should just be made directly through fleet*). Without `task`, the group
 * goes to `to` among the groups; with it, the task goes into `group` at `to`.
 * `to` counts from zero, in the order after the move.
 */
export type PlanMove = { group: string; task?: string; to: number };

/**
 * Reordering by drag and drop, and by `⌥↑` / `⌥↓` on a focused group head or
 * task row. **Absent, nothing is draggable** — a plan past its gate is a
 * record.
 */
export type PlanBoardMove = {
  onMove: (move: PlanMove) => void;
  /** Nothing is live to send it over, or a move is already out. */
  disabled?: boolean;
};

/**
 * Removing a group: its tasks dropped, with one reason, asked in place.
 * **Resolves to `null` where it was taken**, or to what the refusal says.
 */
export type PlanBoardRemove = {
  label: string;
  onRemove: (groupId: string, reason: string) => Promise<string | null>;
  disabled?: boolean;
};

/**
 * Adding a task from a group's head (owner, 30 Sep 2026). **The label is the
 * caller's**, like every other word on this board.
 */
export type PlanBoardAdd = {
  label: string;
  /** Pressed in this group's head: the task goes into this group. */
  onAdd: (groupId: string) => void;
  /** Nothing is live to send it over. */
  disabled?: boolean;
};

/**
 * Proposing a change to the plan's Drone in a person's own words (owner, 30
 * Sep 2026) — on a group, here, and on a task, in `PlanTaskSheet`. **One
 * verb for both**, and one form. The words are the caller's.
 */
export type PlanPropose = {
  /** The control that opens the form, and the field's own name — `Propose a change`. */
  label: string;
  /** The form's send — `Send to the Drone`. */
  send: string;
  /** A change is out and nothing has answered. */
  pending?: boolean;
  /** Nothing is live to send it over. */
  disabled?: boolean;
};

/**
 * The proposal's field and its two buttons, opened in place under the acts
 * that hold its control. **Sending closes it**, as a group ask's dialog
 * closes on its confirm: what the Drone answers is drawn with the plan's
 * revisions, not here. An empty field sends nothing.
 */
export function PlanProposeForm({
  label,
  send,
  pending = false,
  disabled = false,
  onSend,
  onCancel,
}: PlanPropose & { onSend: (instruction: string) => void; onCancel: () => void }) {
  const [instruction, setInstruction] = useState("");
  const empty = instruction.trim() === "";
  return (
    <section className="armada-plan-propose" aria-label={label}>
      <Textarea
        label={label}
        rows={3}
        value={instruction}
        disabled={disabled || pending}
        onChange={(event) => setInstruction(event.target.value)}
      />
      <div className="armada-plan-propose__acts">
        <Button variant="secondary" size="sm" ground="sunken" onClick={onCancel}>
          Cancel
        </Button>
        <Button
          variant="secondary"
          size="sm"
          ground="sunken"
          pending={pending}
          disabled={disabled || empty}
          onClick={() => onSend(instruction.trim())}
        >
          {send}
        </Button>
      </div>
    </section>
  );
}

export type PlanBoardProps = {
  /** The plan's own approach line, as the step recorded it. */
  approach: string;
  groups: readonly PlanBoardGroup[];
  /** The task the inspector is open on, so the row it came from says so. */
  openTaskId?: string;
  onOpenTask?: (taskId: string) => void;
  /**
   * The group controls are offered, so the groups' own head draws the `?` that
   * says what they are. **Once above the cards rather than on each**: what it
   * explains is true of the plan and not of any one group.
   */
  askable?: boolean;
  /** A proposal is out and nothing has answered — Propose a change is off. */
  askPending?: boolean;
  /**
   * Proposing a change to one group. **Absent on a plan past its gate**,
   * which takes no requests.
   */
  propose?: PlanPropose & { onPropose: (groupId: string, instruction: string) => void };
  /** Remove on each group. Absent draws none. */
  remove?: PlanBoardRemove;
  /** Drag and drop. Absent, nothing moves. */
  move?: PlanBoardMove;
  /** Absent draws no add in any group's head. */
  add?: PlanBoardAdd;
};

/** What is being dragged, and which drop target the pointer is over. */
type Dragging = { group: string; task?: string };

/**
 * The drag, the drop and the keys, for one surface — the list, or one group's
 * panel. **Native drag and drop, no library**: nothing in this package drags
 * yet, and none is a dependency. The keyboard's way is `⌥↑` / `⌥↓` on the
 * focused thing, since the row's own `Space` and `Enter` already open it.
 *
 * **It moves nothing itself.** A drop is sent, and the order drawn is the
 * caller's: what Fleet answers, not what was hoped.
 */
export function usePlanMover(groups: readonly PlanBoardGroup[], move: PlanBoardMove | undefined) {
  const [dragging, setDragging] = useState<Dragging | null>(null);
  const [over, setOver] = useState<string | null>(null);
  const live = move !== undefined && move.disabled !== true;

  const send = (next: PlanMove) => {
    if (!live) return;
    const g = groups.findIndex((one) => one.id === next.group);
    if (next.task === undefined) {
      if (g === next.to) return;
    } else {
      const from = groups.find((one) => one.tasks.some((task) => task.id === next.task));
      const at = from?.tasks.findIndex((task) => task.id === next.task);
      if (from?.id === next.group && at === next.to) return;
    }
    move.onMove(next);
  };

  /** `⌥↑` / `⌥↓`: one place. A task at a group's edge crosses into the next group. */
  const keyed = (event: KeyboardEvent, target: Dragging) => {
    if (!live || !event.altKey || (event.key !== "ArrowUp" && event.key !== "ArrowDown")) return;
    event.preventDefault();
    event.stopPropagation();
    const by = event.key === "ArrowUp" ? -1 : 1;
    const g = groups.findIndex((one) => one.id === target.group);
    if (target.task === undefined) {
      const to = g + by;
      if (to >= 0 && to < groups.length) send({ group: target.group, to });
      return;
    }
    const group = groups[g];
    const at = group?.tasks.findIndex((task) => task.id === target.task) ?? -1;
    if (group === undefined || at < 0) return;
    const to = at + by;
    if (to >= 0 && to < group.tasks.length) {
      send({ group: group.id, task: target.task, to });
      return;
    }
    const next = groups[g + by];
    if (next === undefined) return;
    send({ group: next.id, task: target.task, to: by < 0 ? next.tasks.length : 0 });
  };

  const start = (event: DragEvent, target: Dragging) => {
    event.stopPropagation();
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData("text/plain", target.task ?? target.group);
    setDragging(target);
  };
  const end = () => {
    setDragging(null);
    setOver(null);
  };
  const hover = (event: DragEvent, key: string, ok: boolean) => {
    if (!ok) return;
    event.preventDefault();
    event.stopPropagation();
    event.dataTransfer.dropEffect = "move";
    if (over !== key) setOver(key);
  };
  const dropped = (event: DragEvent, next: PlanMove | undefined) => {
    event.preventDefault();
    event.stopPropagation();
    end();
    if (next !== undefined) send(next);
  };

  /** A group's handle — its head on the list. */
  const groupHandle = (group: PlanBoardGroup) =>
    !live
      ? {}
      : {
          draggable: true,
          "data-draggable": "true",
          onDragStart: (event: DragEvent) => start(event, { group: group.id }),
          onDragEnd: end,
        };
  /** What a focused group head does with `⌥↑` / `⌥↓`. */
  const groupKeys = (group: PlanBoardGroup) =>
    !live
      ? {}
      : {
          tabIndex: 0,
          "aria-keyshortcuts": "Alt+ArrowUp Alt+ArrowDown",
          onKeyDown: (event: KeyboardEvent) => keyed(event, { group: group.id }),
        };
  /**
   * A group as a drop target. A group dropped here takes its place; a task
   * dropped on the card rather than on a row goes to the group's end.
   */
  const groupTarget = (group: PlanBoardGroup, at: number) => {
    if (!live) return {};
    const key = `group:${group.id}`;
    return {
      "data-drop-over": over === key ? "true" : undefined,
      onDragOver: (event: DragEvent) => hover(event, key, dragging !== null),
      onDragLeave: () => setOver((was) => (was === key ? null : was)),
      onDrop: (event: DragEvent) =>
        dragging === null
          ? undefined
          : dropped(
          event,
          dragging.task === undefined
            ? { group: dragging.group, to: at }
            : {
                group: group.id,
                task: dragging.task,
                to: group.tasks.filter((task) => task.id !== dragging.task).length,
              },
        ),
    };
  };
  /** A task row: dragged, keyed, and — for another task — dropped on. */
  const taskHandle = (group: PlanBoardGroup, task: PlanBoardTask, at: number) => {
    if (!live) return { row: {}, head: {} };
    const key = `task:${task.id}`;
    return {
      row: {
        draggable: true,
        "data-draggable": "true",
        "data-drop-over": over === key ? "true" : undefined,
        onDragStart: (event: DragEvent) => start(event, { group: group.id, task: task.id }),
        onDragEnd: end,
        onDragOver: (event: DragEvent) => hover(event, key, dragging?.task !== undefined),
        onDragLeave: () => setOver((was) => (was === key ? null : was)),
        onDrop: (event: DragEvent) =>
          dragging?.task === undefined
            ? undefined
            : dropped(event, { group: group.id, task: dragging.task, to: at }),
      },
      head: {
        "aria-keyshortcuts": "Alt+ArrowUp Alt+ArrowDown",
        onKeyDown: (event: KeyboardEvent) => keyed(event, { group: group.id, task: task.id }),
      },
    };
  };
  return { groupHandle, groupKeys, groupTarget, taskHandle };
}

export type PlanMover = ReturnType<typeof usePlanMover>;

function TaskRow({
  task,
  open,
  onOpenTask,
  handle,
}: {
  task: PlanBoardTask;
  open: boolean;
  onOpenTask?: (taskId: string) => void;
  handle?: ReturnType<PlanMover["taskHandle"]>;
}) {
  const body = (
    <>
      <TaskMark state={task.mark} />
      <span className="armada-plan-board__task-id">{task.id}</span>
      <span className="armada-plan-board__task-title">{task.title}</span>
      {task.besideSays === undefined ? null : (
        <span className="armada-plan-board__task-beside">{task.besideSays}</span>
      )}
      {/* The flag, and the whole of what #1530 decided: the task stays done
          and says which later one reached into its files. */}
      {task.touchedSays === undefined ? null : (
        <span className="armada-plan-board__task-touched">{task.touchedSays}</span>
      )}
      {/* **An empty slot stays empty** (owner, 30 Sep 2026: *what are these
          dashes representing?*). The span stays so the columns keep their
          line; a bare figure names itself in a tooltip. `34 turns` already
          does, so it takes none. */}
      <Tooltip asChild label="Model">
        <span className="armada-plan-board__task-model">{task.model}</span>
      </Tooltip>
      <span className="armada-plan-board__task-turns">{task.turnsSays}</span>
      {task.costSays === undefined ? (
        <span className="armada-plan-board__task-cost" />
      ) : (
        <Tooltip asChild label="Cost">
          <span className="armada-plan-board__task-cost">{task.costSays}</span>
        </Tooltip>
      )}
    </>
  );
  return (
    // **Named by its task, because the mark speaks first.** `TaskMark` carries
    // its state as the row's leading text for a screen reader, so a row with no
    // name of its own cannot be reached by the id a plan is discussed in.
    <li
      className="armada-plan-board__task"
      data-mark={task.mark}
      aria-label={`${task.id} ${task.title}`}
      {...handle?.row}
    >
      {onOpenTask === undefined ? (
        <span className="armada-plan-board__task-head">{body}</span>
      ) : (
        <button
          type="button"
          className="armada-plan-board__task-head"
          aria-current={open ? "true" : undefined}
          onClick={() => onOpenTask(task.id)}
          {...handle?.head}
        >
          {body}
          <ChevronRight size={12} strokeWidth={2} aria-hidden />
        </button>
      )}
      {/* Labelled, because the reason is the Drone's own words and a bare
          sentence under a row reads as a note somebody left (owner, 28 Sep). */}
      {task.failedReason === undefined ? null : (
        <span className="armada-plan-board__task-failed">
          <span className="armada-plan-board__task-eyebrow">Why it stopped</span>
          <span>{task.failedReason}</span>
        </span>
      )}
    </li>
  );
}

/**
 * The acts on one group: Remove, which drops its tasks with one reason, and
 * Propose a change, the one ask to the Drone. **Under the tasks and above the
 * boundary**, because they are about the group and the boundary is about what
 * runs after it. One form open at a time.
 */
function GroupAsks({
  group,
  pending,
  remove,
  propose,
  guide,
}: {
  group: PlanBoardGroup;
  pending: boolean;
  remove?: PlanBoardRemove;
  propose?: PlanBoardProps["propose"];
  guide: boolean;
}) {
  const [open, setOpen] = useState<"remove" | "propose" | null>(null);
  const shut = () => setOpen(null);
  return (
    <>
      <div className="armada-plan-board__asks" aria-label={`Ask about group ${group.ordinal}`} role="group">
        {remove === undefined || open === "remove" ? null : (
          <Button size="sm" ground="card" disabled={remove.disabled} onClick={() => setOpen("remove")}>
            {remove.label}
          </Button>
        )}
        {propose === undefined || open === "propose" ? null : (
          <Button
            size="sm"
            ground="card"
            disabled={pending || propose.disabled === true}
            onClick={() => setOpen("propose")}
          >
            {propose.label}
          </Button>
        )}
        {guide ? <GuideMark guide={GUIDE_PLAN_ASKS} /> : null}
      </div>
      {remove === undefined || open !== "remove" ? null : (
        <PlanDropForm
          label={`${remove.label} group ${group.ordinal}`}
          send={remove.label}
          sending="Removing…"
          drop={{
            onDrop: (reason) => remove.onRemove(group.id, reason),
            ...(remove.disabled === undefined ? {} : { disabled: remove.disabled }),
          }}
          onClose={shut}
        />
      )}
      {propose === undefined || open !== "propose" ? null : (
        <PlanProposeForm
          label={propose.label}
          send={propose.send}
          pending={pending}
          {...(propose.disabled === undefined ? {} : { disabled: propose.disabled })}
          onCancel={shut}
          onSend={(instruction) => {
            shut();
            propose.onPropose(group.id, instruction);
          }}
        />
      )}
    </>
  );
}

/**
 * What a group's card holds under its head — the overlap warning, its task
 * rows, its asks and its boundary. **One drawing for the list's card and the
 * graph's group panel** (owner, 30 Sep 2026), so pressing a group on the graph
 * reads what the list already shows. A fragment: the caller lays it out.
 */
export function PlanGroupBody({
  group,
  openTaskId,
  onOpenTask,
  askPending = false,
  remove,
  propose,
  mover,
  guide = false,
}: {
  group: PlanBoardGroup;
  openTaskId?: string;
  onOpenTask?: (taskId: string) => void;
  askPending?: boolean;
  remove?: PlanBoardRemove;
  propose?: PlanBoardProps["propose"];
  /**
   * Dragging a task's row, and `⌥↑` / `⌥↓` on it — the surface's own, shared
   * by every card on the list so a task can be dropped in another. Absent,
   * nothing moves.
   */
  mover?: PlanMover;
  /**
   * Guide 6's `?` at the end of the asks. **The group panel's**, where one
   * group is read alone; the list draws it once above every card instead.
   */
  guide?: boolean;
}) {
  const idle = usePlanMover([group], undefined);
  const moves = mover ?? idle;
  return (
    <>
      {(group.overlaps ?? []).map((overlap) => (
        <PlanOverlap key={overlap.says} says={overlap.says} paths={overlap.paths} />
      ))}
      <ul className="armada-plan-board__tasks" aria-label={`Group ${group.ordinal} tasks`}>
        {group.tasks.map((task, at) => (
          <TaskRow
            key={task.id}
            task={task}
            open={task.id === openTaskId}
            handle={moves.taskHandle(group, task, at)}
            {...(onOpenTask === undefined ? {} : { onOpenTask })}
          />
        ))}
      </ul>
      {remove === undefined && propose === undefined ? null : (
        <GroupAsks
          group={group}
          pending={askPending}
          guide={guide}
          {...(remove === undefined ? {} : { remove })}
          {...(propose === undefined ? {} : { propose })}
        />
      )}
      <GroupBoundary {...group.boundary} />
    </>
  );
}

/**
 * A group's name, and — where the plan can move — what a keyboard focuses to
 * move the group, with the binding in its tooltip.
 */
export function PlanGroupName({
  group,
  mover,
  hint,
}: {
  group: PlanBoardGroup;
  mover: PlanMover;
  /** The tooltip's words, where the group can move. */
  hint: string;
}) {
  const keys = mover.groupKeys(group);
  const name = (
    <CardTitle className="armada-plan-board__group-name" {...keys}>
      Group {group.ordinal}
    </CardTitle>
  );
  return keys.tabIndex === undefined ? (
    name
  ) : (
    <Tooltip asChild label={hint} shortcut={keyFor("move_in_plan")}>
      {name}
    </Tooltip>
  );
}

/**
 * How a group's tasks run, drawn — the Overview Plan card's own `GroupShape`,
 * in the list's head and the panel's alike.
 */
export function PlanGroupShape({ group }: { group: PlanBoardGroup }) {
  return <GroupShape tasks={group.tasks.length} concurrent={group.concurrent} label={group.shapeSays} />;
}

function GroupCard({
  group,
  at,
  mover,
  openTaskId,
  onOpenTask,
  askPending,
  remove,
  propose,
  add,
}: {
  group: PlanBoardGroup;
  at: number;
  mover: PlanMover;
  openTaskId?: string;
  onOpenTask?: (taskId: string) => void;
  askPending: boolean;
  remove?: PlanBoardRemove;
  propose?: PlanBoardProps["propose"];
  add?: PlanBoardAdd;
}) {
  return (
    // **Named, because a card now carries other groups' ordinals.** An overlap
    // warning says `Group 4 writes these files too`, so a reader — and a test —
    // looking for group 4 by its text reaches the card that mentions it first.
    <li className="armada-plan-board__group" aria-label={`Group ${group.ordinal}`} {...mover.groupTarget(group, at)}>
      <Card data-state={group.state}>
        {/* **The head is the handle** — the icon registry has no grip, so
            the row itself is what is dragged, and its name is what a
            keyboard focuses to move it. */}
        <CardHeader className="armada-plan-board__group-head" {...mover.groupHandle(group)}>
          <PlanGroupName group={group} mover={mover} hint="Drag to move" />
          <PlanGroupShape group={group} />
          <PlanGroupStateWord state={group.state} says={group.says} />
          <span className="armada-plan-board__group-gap" />
          <span className="armada-plan-board__scope" title={group.scope.root}>
            <span className="armada-plan-board__scope-root">{group.scope.root}</span>
          </span>
          {add === undefined ? null : (
            <Button
              variant="ghost"
              size="sm"
              ground="card"
              disabled={add.disabled}
              onClick={() => add.onAdd(group.id)}
            >
              {add.label}
            </Button>
          )}
        </CardHeader>
        <CardContent>
          <PlanGroupBody
            group={group}
            askPending={askPending}
            mover={mover}
            {...(openTaskId === undefined ? {} : { openTaskId })}
            {...(onOpenTask === undefined ? {} : { onOpenTask })}
            {...(remove === undefined ? {} : { remove })}
            {...(propose === undefined ? {} : { propose })}
          />
        </CardContent>
      </Card>
    </li>
  );
}

export function PlanBoard({
  approach,
  groups,
  openTaskId,
  onOpenTask,
  askable = false,
  askPending = false,
  propose,
  remove,
  move,
  add,
}: PlanBoardProps) {
  const mover = usePlanMover(groups, move);
  return (
    <div className="armada-plan-board">
      <section className="armada-plan-board__approach" aria-label="The approach">
        <Clamped lines={3}>{approach}</Clamped>
      </section>
      <div className="armada-plan-board__group-region">
        {/* The noun and nothing else — `Groups` is true of a plan that has
            never run (#1602). Guide 4 hangs here because the list is named
            for the order the groups run in. */}
        <div className="armada-plan-board__groups-head">
          <h3 className="armada-plan-board__groups-title">Groups</h3>
          <GuideMark guide={GUIDE_GROUP_ORDER} />
          {askable ? <GuideMark guide={GUIDE_PLAN_ASKS} /> : null}
        </div>
        <ol className="armada-plan-board__groups" aria-label="Groups, in the order they run">
          {groups.map((group, at) => (
            <GroupCard
              key={group.id}
              group={group}
              at={at}
              mover={mover}
              askPending={askPending}
              {...(openTaskId === undefined ? {} : { openTaskId })}
              {...(onOpenTask === undefined ? {} : { onOpenTask })}
              {...(remove === undefined ? {} : { remove })}
              {...(propose === undefined ? {} : { propose })}
              {...(add === undefined ? {} : { add })}
            />
          ))}
        </ol>
      </div>
    </div>
  );
}
