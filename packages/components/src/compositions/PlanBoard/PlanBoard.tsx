import { ChevronRight, TriangleAlert } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import { Card, CardContent, CardHeader, CardTitle } from "../../primitives/Card/Card";
import { Clamped } from "../Clamped/Clamped";
import { GroupBoundary, type GroupBoundaryProps } from "../GroupBoundary/GroupBoundary";
import { GuideMark } from "../GuideMark/GuideMark";
import { GUIDE_GROUP_ORDER, GUIDE_PLAN_ASKS } from "../../guides";
import { TaskMark, type TaskMarkState } from "../TaskMark/TaskMark";

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
 * Where this group writes, as one value. `count` is how many paths it claims,
 * which is what the root stops saying.
 */
export type PlanBoardScope = { root: string; count: number };

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

export type PlanBoardGroup = {
  id: string;
  /** Its position in the step, counted from one. */
  ordinal: number;
  state: PlanGroupState;
  /** Where the group is, in words — `working`, `passed`, `failed at its checks`. */
  says: string;
  /** `2 tasks, at the same time`, `4 tasks, one after another`. */
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
  /**
   * What may be asked of the Drone about this group. **Empty draws nothing**,
   * which is every plan already approved: the plan is a record then, and a
   * record takes no requests.
   */
  asks?: readonly PlanBoardAsk[];
};

/**
 * One change offered on a group. **The label is the caller's**, like every
 * other word on this board — `id` is what comes back, never what is drawn.
 */
export type PlanBoardAsk = {
  /** What comes back to `onAsk`, in the caller's own vocabulary. */
  id: string;
  label: string;
  /** Off where the plan cannot take it — a first group has no group above it. */
  disabled?: boolean;
};

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
  /** A group ask was pressed and nothing has answered — every ask is off. */
  askPending?: boolean;
  onAsk?: (groupId: string, askId: string) => void;
};

/** What a column holds where the record holds nothing yet. */
const EMPTY = "—";

function TaskRow({
  task,
  open,
  onOpenTask,
}: {
  task: PlanBoardTask;
  open: boolean;
  onOpenTask?: (taskId: string) => void;
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
      <span className="armada-plan-board__task-model">{task.model}</span>
      <span className="armada-plan-board__task-turns">{task.turnsSays ?? EMPTY}</span>
      <span className="armada-plan-board__task-cost">{task.costSays ?? EMPTY}</span>
    </>
  );
  return (
    // **Named by its task, because the mark speaks first.** `TaskMark` carries
    // its state as the row's leading text for a screen reader, so a row with no
    // name of its own cannot be reached by the id a plan is discussed in.
    <li className="armada-plan-board__task" data-mark={task.mark} aria-label={`${task.id} ${task.title}`}>
      {onOpenTask === undefined ? (
        <span className="armada-plan-board__task-head">{body}</span>
      ) : (
        <button
          type="button"
          className="armada-plan-board__task-head"
          aria-current={open ? "true" : undefined}
          onClick={() => onOpenTask(task.id)}
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
 * What may be asked of the Drone about one group.
 *
 * **Under the card's own content and above the boundary**, because an ask is
 * about the group and the boundary is about what runs after it.
 */
function GroupAsks({
  group,
  pending,
  onAsk,
}: {
  group: PlanBoardGroup;
  pending: boolean;
  onAsk: (groupId: string, askId: string) => void;
}) {
  return (
    <div className="armada-plan-board__asks" aria-label={`Ask about group ${group.ordinal}`} role="group">
      {group.asks?.map((ask) => (
        <Button
          key={ask.id}
          size="sm"
          ground="card"
          disabled={pending || ask.disabled === true}
          onClick={() => onAsk(group.id, ask.id)}
        >
          {ask.label}
        </Button>
      ))}
    </div>
  );
}

function GroupCard({
  group,
  openTaskId,
  onOpenTask,
  askPending,
  onAsk,
}: {
  group: PlanBoardGroup;
  openTaskId?: string;
  onOpenTask?: (taskId: string) => void;
  askPending: boolean;
  onAsk?: (groupId: string, askId: string) => void;
}) {
  return (
    <li className="armada-plan-board__group">
      <Card data-state={group.state}>
        <CardHeader className="armada-plan-board__group-head">
          <CardTitle className="armada-plan-board__group-name">Group {group.ordinal}</CardTitle>
          <span className="armada-plan-board__shape">{group.shapeSays}</span>
          <span className="armada-plan-board__state" data-state={group.state}>
            {group.says}
          </span>
          <span className="armada-plan-board__group-gap" />
          <span className="armada-plan-board__scope" title={group.scope.root}>
            <span className="armada-plan-board__scope-root">{group.scope.root}</span>
            <span className="armada-plan-board__scope-count">{group.scope.count}</span>
          </span>
        </CardHeader>
        <CardContent>
          {(group.overlaps ?? []).map((overlap) => (
            <div className="armada-plan-board__overlap" key={overlap.says} role="note">
              <TriangleAlert size={12} strokeWidth={2} aria-hidden />
              <span className="armada-plan-board__overlap-says">{overlap.says}</span>
              <ul className="armada-plan-board__overlap-paths">
                {overlap.paths.map((path) => (
                  <li key={path}>{path}</li>
                ))}
              </ul>
            </div>
          ))}
          <ul className="armada-plan-board__tasks" aria-label={`Group ${group.ordinal} tasks`}>
            {group.tasks.map((task) => (
              <TaskRow
                key={task.id}
                task={task}
                open={task.id === openTaskId}
                {...(onOpenTask === undefined ? {} : { onOpenTask })}
              />
            ))}
          </ul>
          {onAsk === undefined || (group.asks ?? []).length === 0 ? null : (
            <GroupAsks group={group} pending={askPending} onAsk={onAsk} />
          )}
          <GroupBoundary {...group.boundary} />
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
  onAsk,
}: PlanBoardProps) {
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
          {groups.map((group) => (
            <GroupCard
              key={group.id}
              group={group}
              askPending={askPending}
              {...(openTaskId === undefined ? {} : { openTaskId })}
              {...(onOpenTask === undefined ? {} : { onOpenTask })}
              {...(onAsk === undefined ? {} : { onAsk })}
            />
          ))}
        </ol>
      </div>
    </div>
  );
}
