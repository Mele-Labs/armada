import { useState } from "react";
import { ChevronRight, TriangleAlert } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import { Card, CardContent, CardHeader, CardTitle } from "../../primitives/Card/Card";
import { Clamped } from "../Clamped/Clamped";
import { GroupBoundary, type GroupBoundaryProps } from "../GroupBoundary/GroupBoundary";
import { GuideMark } from "../GuideMark/GuideMark";
import { GUIDE_GROUP_ORDER, GUIDE_PLAN_ASKS } from "../../guides";
import { TaskMark, type TaskMarkState } from "../TaskMark/TaskMark";
import { Textarea } from "../../primitives/Textarea/Textarea";

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
  /** A group ask was pressed and nothing has answered — every ask is off. */
  askPending?: boolean;
  onAsk?: (groupId: string, askId: string) => void;
  /**
   * Proposing a change to one group, beside its asks. **Drawn on a group
   * that offers asks and on no other**: a plan past its gate takes no
   * requests.
   */
  propose?: PlanPropose & { onPropose: (groupId: string, instruction: string) => void };
  /** Absent draws no add in any group's head. */
  add?: PlanBoardAdd;
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
  propose,
  guide,
}: {
  group: PlanBoardGroup;
  pending: boolean;
  onAsk: (groupId: string, askId: string) => void;
  propose?: PlanBoardProps["propose"];
  guide: boolean;
}) {
  const [proposing, setProposing] = useState(false);
  return (
    <>
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
        {propose === undefined || proposing ? null : (
          <Button
            size="sm"
            ground="card"
            disabled={pending || propose.disabled === true}
            onClick={() => setProposing(true)}
          >
            {propose.label}
          </Button>
        )}
        {guide ? <GuideMark guide={GUIDE_PLAN_ASKS} /> : null}
      </div>
      {propose === undefined || !proposing ? null : (
        <PlanProposeForm
          label={propose.label}
          send={propose.send}
          pending={pending}
          {...(propose.disabled === undefined ? {} : { disabled: propose.disabled })}
          onCancel={() => setProposing(false)}
          onSend={(instruction) => {
            setProposing(false);
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
  onAsk,
  propose,
  guide = false,
}: {
  group: PlanBoardGroup;
  openTaskId?: string;
  onOpenTask?: (taskId: string) => void;
  askPending?: boolean;
  onAsk?: (groupId: string, askId: string) => void;
  propose?: PlanBoardProps["propose"];
  /**
   * Guide 6's `?` at the end of the asks. **The group panel's**, where one
   * group is read alone; the list draws it once above every card instead.
   */
  guide?: boolean;
}) {
  return (
    <>
      {(group.overlaps ?? []).map((overlap) => (
        <PlanOverlap key={overlap.says} says={overlap.says} paths={overlap.paths} />
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
        <GroupAsks
          group={group}
          pending={askPending}
          onAsk={onAsk}
          guide={guide}
          {...(propose === undefined ? {} : { propose })}
        />
      )}
      <GroupBoundary {...group.boundary} />
    </>
  );
}

function GroupCard({
  group,
  openTaskId,
  onOpenTask,
  askPending,
  onAsk,
  propose,
  add,
}: {
  group: PlanBoardGroup;
  openTaskId?: string;
  onOpenTask?: (taskId: string) => void;
  askPending: boolean;
  onAsk?: (groupId: string, askId: string) => void;
  propose?: PlanBoardProps["propose"];
  add?: PlanBoardAdd;
}) {
  return (
    // **Named, because a card now carries other groups' ordinals.** An overlap
    // warning says `Group 4 writes these files too`, so a reader — and a test —
    // looking for group 4 by its text reaches the card that mentions it first.
    <li className="armada-plan-board__group" aria-label={`Group ${group.ordinal}`}>
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
            {...(openTaskId === undefined ? {} : { openTaskId })}
            {...(onOpenTask === undefined ? {} : { onOpenTask })}
            {...(onAsk === undefined ? {} : { onAsk })}
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
  onAsk,
  propose,
  add,
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
              {...(propose === undefined ? {} : { propose })}
              {...(add === undefined ? {} : { add })}
            />
          ))}
        </ol>
      </div>
    </div>
  );
}
