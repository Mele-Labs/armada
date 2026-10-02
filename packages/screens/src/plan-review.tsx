// A plan's review: its Graph and List, the group and task panels, and every
// act a person takes on a plan before it runs.
//
// **Plan's own surface, mounted in two places** (owner, 30 Sep 2026). The
// Plan destination draws it under its lead, and Overview's review gate draws
// it where a waiting step claimed a plan — `verdictSlotAtGate` switches on
// the claim's `evidence_type`. One region and one call, so the acts at the
// two cannot drift: the cost the owner took was a second copy of Plan's
// review, on the condition that it stays the same one.
//
// **It holds its own open state and nothing of its host's.** Which panel is
// open, what is typed and which dialog is up are this region's; the gate's
// acts, its pending press and its answer are not, and reach it only as the
// props below.

import {
  DroneBrief,
  PlanBoard,
  type PlanMove,
  PlanGroupSheet,
  PlanTaskSheet,
  Tabs,
  WorkflowCanvas,
} from "@armada/components";
import { useEffect, useMemo, useState, type ReactNode } from "react";

import type {
  Diff,
  EditTask,
  JobDetail as JobWhole,
  JobSummary,
  JudgeAnswer,
  MovePlan,
  Outcome,
  StepDetail,
} from "@armada/protocol";

import { PlanGate } from "./plan-lead";
import type { ActingAct } from "./pending";
import { casesOf, droneOfTask, groupsOf, PROPOSE_ASK, REMOVE_GROUP_LABEL, taskSheetOf, tasksOf } from "./tab-plan-read";
import { movedGroups, moveSent, planBoardOf } from "./plan-board";
import { steeringOf } from "./steering";
import { stepThatWorksTheGroups } from "./workflow-canvas";
import { proposeInstruction, rewriteInstruction } from "./tab-plan-ask";
import { planGraphOf, taskCard } from "./plan-canvas";
import { PLAN_VIEWS, PLAN_VIEW_LABEL, type PlanView } from "./plan-view";
import { CHANGED_NOTHING, drawn } from "./review";
import type { ConfirmableAct, HeldAct, TaskAct } from "./Acts";
import { TASK_STOP } from "./copy";
import { doingOfTask, hasOwnDrone, lastEditOf } from "./task-live";
import type { JobDraft } from "./draft/held";
import type { GroupView } from "./draft/group";
import { taskDronesOf, type DroneView } from "./draft/drone";
import { spentOf } from "./workflow-inspector";
import {
  DRONE_SAYS,
  droneOnTask,
  droneTurnsOf,
  ranForOf,
  TRANSCRIPT_EMPTY,
  TRANSCRIPT_UNSERVED,
} from "./tab-drones-read";
import type { TrailProps } from "./trail";
import { ADD_TASK_LABEL } from "./copy";
import type { AddTask, DropTask, PlanEditAnswer } from "./plan-edits";
import { AddTaskDialog, refusalSaid } from "./PlanWell";

/** The `DeclaredCheck.kind` a step recording a plan declares. `plan.ts`'s own read. */
const PLAN_RECORDED = "plan_recorded";

export type PlanReviewProps = {
  job: JobSummary;
  whole: JobWhole | null;
  /** What this moment's boards draw that Fleet cannot serve yet. */
  draft?: JobDraft;
  /** The window is at `--window-floor`, so the panels go flush. */
  floor: boolean;
  /**
   * Which arrangement the plan is in, and the press that moves it. **Remembered
   * per viewer by the caller, and one choice for both hosts** — the Plan
   * destination and the gate draw one plan, and a toggle that differed between
   * them would be two readings of it.
   */
  view: PlanView;
  onView: (view: PlanView) => void;
  /** Every control is refused while what is shown is not live. */
  stale: boolean;
  /** An act on this Job is already out. */
  acting: boolean;
  /** A decision at this gate is already out. */
  deciding: boolean;
  /** Which act is out, so the Judge's answer that was pressed waits. */
  actingAct?: ActingAct | undefined;
  onApproveReview: (jobId: string) => void;
  /**
   * Answer a Judge's refusal on the plan's step — Overview's own handler, for
   * the block the gate draws in place of Approve the plan while one is open.
   */
  onAnswerJudge: (jobId: string, askedAt: string, answer: JudgeAnswer, note?: string) => void;
  /**
   * The Board's own rows. **Where one waits at `awaiting_approval` dispatched
   * by this Job, the plan is an Epic's and those are its proposed wave**
   * (the decision that approving an Epic's plan releases its wave),
   * so Approve the plan releases them together through `onApproveWave`.
   */
  board?: readonly JobSummary[];
  /**
   * Approve an Epic's plan and release every Job of its proposed wave, in one
   * act. Ahead of its route (#1694). Absent, the gate approves as any plan's.
   */
  onApproveWave?: (jobId: string, jobs: readonly string[]) => void;
  onRedirect: (jobId: string, instruction: string) => void;
  /**
   * A failed task's Pilot or Restart, and Edit this task with what it
   * changed. **The buttons are drawn without it**, so the owner can read
   * them; a press does nothing until the host hands this through.
   */
  onTaskAct?: (act: TaskAct, jobId: string, taskId: string, edit?: EditTask) => Promise<Outcome>;
  /**
   * What Hold to stop this task sends, held and asked. **Absent draws no
   * stop**: the host that hands these through is the one with a confirmation
   * to ask with.
   */
  onAct?: (act: ConfirmableAct, jobId: string) => void;
  onActHeld?: (act: HeldAct, jobId: string) => void;
  /**
   * Every model the app knows — `list_models` — which Edit this task picks
   * from (owner, 30 Sep 2026). Absent offers the task's own model alone.
   */
  models?: readonly string[];
  /**
   * Add a task to the plan, from a group's head, and drop one, from its
   * panel, with a reason (owner, 30 Sep 2026) — `PlanWell`'s own two routes.
   * **Absent draws neither.** `after` is the group's last task; where a task
   * goes after that is a move, below. Remove on a group drops each of its
   * tasks through `onDropTask`, with one reason.
   */
  onAddTask?: (jobId: string, add: AddTask) => Promise<PlanEditAnswer>;
  onDropTask?: (jobId: string, drop: DropTask) => Promise<PlanEditAnswer>;
  /**
   * A group or a task dragged somewhere new, sent straight to Fleet (owner,
   * 30 Sep 2026: *edits to the plan should just be made directly through
   * fleet*). **Absent, nothing is draggable.** #1685, served since 23.4.
   */
  onMovePlan?: (jobId: string, move: MovePlan) => Promise<Outcome>;
  /** What an add or a drop that was taken says, once. */
  onSaid?: (sentence: string) => void;
  /**
   * The Job's patch, which a file in the task panel opens to (owner, 29 Sep
   * 2026). **Absent draws no file as a press.** Holding the read open is the
   * host destination's, not this region's.
   */
  diff?: Diff;
  /**
   * The task to land on with its panel open — a task pressed in the Drones
   * sheet. Read once, when the region mounts; after that the panel is the person's.
   */
  opensTask?: string;
  /** Open a Drone in the Drones destination, with its sheet open. Absent, the peek draws no Open. */
  onOpenDrone?: (droneId: string) => void;
  /**
   * Every Drone the Job has had — the Drones destination's own list, so a
   * task's panel and that tab show one Drone the same way. **Absent reads the
   * draft's**, and then the working task's from the plan alone.
   */
  drones?: readonly DroneView[];
  /** Now, injected, so a running Drone's run time moves with the header's. */
  now?: number;
  /**
   * Open a boundary Check's own row in the Record, by its name and the step
   * attempt that ran it. **The screen's.** Absent, no Check is a button.
   */
  onOpenCheck?: (name: string, stepAttempt: number, group?: string) => void;
  /**
   * The way back, where a press in another destination's panel landed here,
   * and where this one's open panel is reported — `trail.ts`.
   */
  trail?: TrailProps;
};

/**
 * Which step recorded the plan, where one did.
 *
 * **The plan's own `recorded_by` first, and the declared check after it.**
 * `plan.ts`'s rule is that the workflow's name never decides this, and both
 * readings obey it: a recorded plan says which run of which step wrote it,
 * which is the exact fact, and a plan not yet recorded has only the step that
 * declares it will. `#1006` is why the second is a check and not a name.
 */
export function planStepOf(whole: JobWhole | null): StepDetail | undefined {
  if (whole === null) return undefined;
  const wrote = whole.work_plan?.recorded_by;
  if (wrote !== undefined && wrote.by === "step") {
    const step = whole.steps.find((one) => one.step_id === wrote.step_id);
    if (step !== undefined) return step;
  }
  return whole.steps.find((step) =>
    (step.checks ?? []).some((check) => check.kind === PLAN_RECORDED),
  );
}

/**
 * The review, in the three pieces a host places. **`gate`** is Approve the
 * plan and Propose a change, drawn only while the plan waits; **`region`** is
 * the Graph/List toggle and the plan under it, `undefined` where there is no
 * plan to draw; **`layers`** are the panels and dialogs, which a host puts
 * outside the box that scrolls.
 */
export type PlanReviewParts = {
  step: StepDetail | undefined;
  groups: readonly GroupView[];
  gate: ReactNode;
  region: ReactNode | undefined;
  layers: ReactNode;
};

export function usePlanReview({
  job,
  whole,
  draft,
  floor,
  view,
  onView,
  stale,
  acting,
  deciding,
  actingAct,
  onApproveReview,
  onAnswerJudge,
  board: rows = [],
  onApproveWave,
  onRedirect,
  onTaskAct,
  onAct,
  onActHeld,
  models = [],
  onAddTask,
  onDropTask,
  onMovePlan,
  onSaid,
  diff,
  opensTask,
  onOpenDrone,
  drones,
  now,
  onOpenCheck,
  trail,
}: PlanReviewProps): PlanReviewParts {
  // Which task the inspector is on. **This region's own state, not the
  // screen's** — the sheet belongs to the destination, so a reader who leaves
  // and comes back lands on the board rather than inside one task.
  const [openTask, setOpenTask] = useState<string | null>(opensTask ?? null);
  // A move that is out and not answered. **Drawn where it was dropped until
  // Fleet answers**, then the plan is whatever Fleet says — so a refused move
  // snaps back rather than sitting in an order nothing holds.
  const [moving, setMoving] = useState<PlanMove | null>(null);
  // Which task the add-task dialog adds after, and the ordinal of the group it
  // adds into, or `null` while it is shut. A group's Add task adds into that
  // group (owner, 30 Sep 2026): after its last task, or for a group with none,
  // after the nearest task before it — `""`, the plan's end, only where no
  // task comes before.
  const [adding, setAdding] = useState<{ after: string; group?: number } | null>(null);
  // What has been typed at the open task's Drone and not sent. This region's
  // own state, on the sheet's terms: it goes when the sheet does.
  const [instruction, setInstruction] = useState("");
  // The file open beside the task. It belongs to the task: another task, or
  // none, closes it.
  const [openFile, setOpenFile] = useState<string | null>(null);
  // The group whose panel is open, pressed on the graph. **One panel at a
  // time**: opening a task closes it, and opening it closes the task.
  const [openGroup, setOpenGroup] = useState<string | null>(null);
  // The group whose panel the open task was pressed in, so Close and Back on
  // the task land on that group again (owner, 30 Sep 2026). **This region's
  // own one step back**, not the trail: the trail is for a jump between
  // destinations, and this never leaves the plan.
  const [fromGroup, setFromGroup] = useState<string | null>(null);
  const openTaskAt = (id: string | null) => {
    setOpenTask(id);
    setOpenFile(null);
    setOpenGroup(null);
    setFromGroup(null);
  };
  const openGroupAt = (id: string) => {
    openTaskAt(null);
    setInstruction("");
    setOpenGroup(id);
  };

  // Split once per reading, not per render: a patch re-split on every tick is
  // the freeze `Sheets.tsx`' own rail was moved off.
  const patch = useMemo(
    () =>
      diff === undefined || diff.state !== "read" || diff.jobId !== job.id || diff.work === undefined
        ? undefined
        : drawn(diff.work),
    [diff, job.id],
  );

  const step = planStepOf(whole);
  // **Only while the step that recorded the plan is waiting on a person, and
  // only while the window is live.** Past that gate the plan is a record, and
  // a stale window cannot tell whether the ask would still land.
  const revisable = step?.state === "awaiting_human" && !stale;

  const groups = groupsOf(whole, draft);
  const addInto = (groupId: string) => {
    const upTo = groups.slice(0, groups.findIndex((one) => one.id === groupId) + 1);
    const group = upTo.at(-1)?.ordinal;
    setAdding({
      after: upTo.flatMap((one) => one.tasks).at(-1)?.id ?? "",
      ...(group === undefined ? {} : { group }),
    });
  };
  const cases = casesOf(whole, draft);
  // **The step the groups are worked at, for the failed Check's own output.**
  // The board draws what each boundary came to now that no second board does
  // (owner, 28 Sep 2026), and a Check result lives on that step's `check_runs`.
  const worksAt =
    whole === null ? undefined : whole.steps.find((one) => one.step_id === stepThatWorksTheGroups(whole));
  const read = planBoardOf(whole, draft, openTaskAt, openTask ?? undefined, revisable, worksAt, onOpenCheck);
  const board = read === undefined || moving === null ? read : { ...read, groups: movedGroups(read.groups, moving) };
  // The same plan, placed. **One press for one task either way** — a toggle
  // that opened a different surface from each view would be two screens.
  const graph = planGraphOf({ groups, onOpenTask: openTaskAt, openTask, onOpenGroup: openGroupAt, openGroup });
  const group = openGroup === null ? undefined : board?.groups.find((one) => one.id === openGroup);
  const cameFrom = fromGroup === null ? undefined : board?.groups.find((one) => one.id === fromGroup);
  const toGroup =
    cameFrom === undefined
      ? undefined
      : {
          label: `Back to Group ${cameFrom.ordinal}`,
          tooltip: `Back to Group ${cameFrom.ordinal}`,
          onBack: () => openGroupAt(cameFrom.id),
        };
  // Proposing a change to one group, on the list's card and in its panel
  // alike — the group asks' own gate and pending.
  const propose = revisable
    ? {
        ...PROPOSE_ASK,
        onPropose: (groupId: string, instruction: string) =>
          onRedirect(job.id, proposeInstruction(groups, groupId, instruction)),
      }
    : undefined;
  // **A person's own edits, direct, while the plan waits on them** — the
  // gate Propose a change takes, and not Add task's: Add and Drop are offered
  // on a running plan too, and a running plan's order is no longer the
  // person's to rearrange. The owner's decision of 30 Sep 2026, *plan edits go
  // straight through Fleet*.
  const move =
    !revisable || onMovePlan === undefined
      ? undefined
      : {
          onMove: (next: PlanMove) => {
            setMoving(next);
            void onMovePlan(job.id, moveSent(read?.groups ?? [], next)).finally(() => setMoving(null));
          },
          disabled: moving !== null,
        };
  // Remove on a group: each of its tasks still on the plan dropped with the
  // one reason, through the drop Fleet already serves. The first refusal
  // stops it and is what the form says.
  const remove =
    !revisable || onDropTask === undefined
      ? undefined
      : {
          label: REMOVE_GROUP_LABEL,
          onRemove: async (groupId: string, reason: string) => {
            const held = groups.find((one) => one.id === groupId)?.tasks ?? [];
            for (const task of held) {
              if (task.state === "done" || task.state === "dropped") continue;
              const answer = await onDropTask(job.id, { task: task.id, reason });
              if (!answer.ok) return refusalSaid(answer.outcome);
            }
            onSaid?.("Removed");
            return null;
          },
          disabled: stale,
        };
  const reading = openTask === null ? undefined : taskSheetOf(openTask, groups, cases);
  useEffect(() => trail?.onHere(reading === undefined ? null : { id: reading.id, label: reading.id }), [reading?.id]);
  // What it runs beside, as the graph's own card off the same groups, so each
  // reads the task's state now. Pressing one opens it here.
  const beside =
    reading === undefined
      ? []
      : tasksOf(groups)
          .filter((one) => reading.beside.includes(one.id))
          .map((one) => taskCard(one, () => openTaskAt(one.id)));
  // **A failed task offers four acts** (owner, 29 Sep 2026): the message box
  // below, these two, and Edit this task, each ahead of its route. A done task
  // in a group the Judge refused offers Restart alone (owner, 2 Oct 2026).
  const refused =
    reading?.state === "done" &&
    groups.some((group) => group.judge_refused === true && group.tasks.some((task) => task.id === reading.id));
  const acts =
    reading === undefined
      ? undefined
      : reading.state === "failed"
        ? {
            onPilot: () => void onTaskAct?.("pilot_task", job.id, reading.id),
            onRestart: () => void onTaskAct?.("restart_task", job.id, reading.id),
            disabled: stale,
          }
        : refused
          ? { onRestart: () => void onTaskAct?.("restart_task", job.id, reading.id), disabled: stale }
          : undefined;
  // **Edit this task, on a task nothing is working on yet or any more**: open
  // or failed. A working task's Drone is mid-way through what the fields say,
  // and a done or dropped one has nothing left to change (owner, 30 Sep 2026).
  const edit =
    reading === undefined || (reading.state !== "open" && reading.state !== "failed")
      ? undefined
      : {
          models,
          onEdit: async (changed: EditTask) => {
            const answer = await onTaskAct?.("edit_task", job.id, reading.id, changed);
            return answer?.ok === true;
          },
          disabled: stale,
        };
  // **Offered on a task that can still be dropped**: not done, which has
  // nothing left to drop, and not dropped, which already is one. A failed
  // task is still on the plan, so it can be.
  const drop =
    reading === undefined || onDropTask === undefined || reading.state === "done" || reading.state === "dropped"
      ? undefined
      : {
          onDrop: async (reason: string) => {
            const answer = await onDropTask(job.id, { task: reading.id, reason });
            if (!answer.ok) return refusalSaid(answer.outcome);
            onSaid?.("Dropped");
            return null;
          },
          disabled: stale,
        };
  const proposeTask =
    reading === undefined || !revisable
      ? undefined
      : {
          ...PROPOSE_ASK,
          pending: acting,
          onPropose: (instruction: string) =>
            onRedirect(job.id, rewriteInstruction(reading.id, instruction)),
        };
  // **Telling this task's own Drone something, in the surface the task is read
  // in** — review and reply are one loop (`#1536`). Drawn only past the gate: a
  // plan still waiting on a person offers the rewrite ask above instead, and two
  // boxes about one task would be two ways to say the same thing to nobody.
  // (`proposeTask` is that ask.)
  const open = openTask === null ? undefined : tasksOf(groups).find((one) => one.id === openTask);
  // **The task's own Drone, off the list the Drones destination reads**, so
  // the peek and that sheet show one Drone the same way.
  const held = drones ?? draft?.drones ?? taskDronesOf(groups);
  const own = open === undefined ? undefined : droneOnTask(held, open.id);
  // **Every Drone the task has had**, oldest first, each opening where the
  // Drones tab opens it (owner, 2 Oct 2026: *why is the task panel for a plan
  // not showing the drones?*). The panel is shared by Graph and List.
  const taskDrones =
    open === undefined
      ? []
      : held
          .filter((one) => one.task === open.id)
          .sort((a, b) => (a.since ?? "").localeCompare(b.since ?? ""))
          .map((one) => {
            const spent = spentOf(one).join(" · ");
            return {
              id: one.id,
              label: droneOfTask(whole, { ...open, drone_id: one.id })?.label ?? `Drone on ${open.id}`,
              state: one.state,
              stateSays: DRONE_SAYS[one.state],
              ...(one.model === undefined ? {} : { model: one.model }),
              ...(spent === "" ? {} : { spent }),
              ...(onOpenDrone === undefined ? {} : { onOpen: () => onOpenDrone(one.id) }),
            };
          });
  const drone = open === undefined || revisable ? undefined : droneOfTask(whole, open);
  const steering = steeringOf(job, whole);
  const peekTurns = useMemo(
    () =>
      own?.transcript === undefined
        ? []
        : droneTurnsOf(own.transcript, (lines) => <DroneBrief lines={lines} flat />),
    [own],
  );
  const ran = own === undefined || now === undefined ? undefined : ranForOf(own, now);
  const peek =
    own === undefined || open === undefined
      ? undefined
      : {
          title: droneOfTask(whole, { ...open, drone_id: own.id })?.label ?? `Drone on ${open.id}`,
          state: own.state,
          stateSays: DRONE_SAYS[own.state],
          ...(ran === undefined ? {} : { ranFor: ran }),
          turns: peekTurns,
          live: own.state === "running",
          emptyNote: own.transcript === undefined ? TRANSCRIPT_UNSERVED : TRANSCRIPT_EMPTY,
          ...(onOpenDrone === undefined ? {} : { onOpen: () => onOpenDrone(own.id) }),
        };
  // **What only a task's own Drone can say: what it is doing, what it last
  // wrote, and a stop for it alone.** Only on a step working a Drone per task
  // (23.1), where the Drone listed on the task is its own; elsewhere the sheet
  // draws none of the three, never the Job's Drone under the task's name.
  const ownDrone =
    open !== undefined && (hasOwnDrone(open) || (open.treatment === "own_drone" && own !== undefined));
  const doing = open === undefined ? undefined : doingOfTask(open);
  const lastEdit = ownDrone ? lastEditOf(own?.transcript) : undefined;
  // **The stop is mocked**: Fleet has no act that ends one task's Drone
  // (#1666, slice 5), so this sends the Job's `kill_drone`, as the Drones
  // sheet's per-Drone kill already does. With one Drone per Job, that is the
  // same Drone.
  const stop =
    !ownDrone || own?.state !== "running" || onAct === undefined || onActHeld === undefined
      ? undefined
      : {
          children: TASK_STOP.label,
          askLabel: TASK_STOP.ask,
          description: TASK_STOP.said,
          disabled: stale || steering.act === undefined || (acting && actingAct !== "kill_drone"),
          pending: acting && actingAct === "kill_drone",
          onAsk: () => onAct("kill_drone", job.id),
          onCommit: () => onActHeld("kill_drone", job.id),
        };
  // **An open task no Drone has run has nothing to reach**: the Job's Drone is
  // not on it, so the panel draws no box rather than one that lands elsewhere.
  const redirect =
    reading === undefined || drone === undefined || (own === undefined && reading.state === "open")
      ? undefined
      : {
          value: instruction,
          onChange: setInstruction,
          onSend: () => {
            onRedirect(job.id, instruction);
            setInstruction("");
          },
          disabled: stale || steering.act === undefined,
          disabledReason: NO_DRONE,
          ...(steering.sent === undefined ? {} : { waiting: steering.sent }),
        };

  // Everything the Job did to the open file. **The Job's patch, not the
  // task's** — where two tasks wrote one file, both show.
  const patched = patch?.files.map((one) => one.path);
  const drawnFile = openFile === null ? undefined : patch?.files.find((one) => one.path === openFile);
  const last = patch?.files[patch.files.length - 1];
  const file =
    drawnFile === undefined || patch === undefined
      ? undefined
      : {
          path: drawnFile.path,
          diff: {
            files: [drawnFile],
            emptyNote: CHANGED_NOTHING,
            // The bound cuts the last file drawn and no other.
            ...(patch.cut === undefined || last !== drawnFile ? {} : { cut: patch.cut }),
          },
        };

  // The wave this plan proposed, where it is an Epic's: every Job it
  // dispatched that is waiting to be released. One press releases them all.
  const wave = rows
    .filter((row) => row.dispatched_by === job.id && row.status === "awaiting_approval")
    .map((row) => row.id);

  const gate = (
    <PlanGate
      job={job}
      whole={whole}
      step={step}
      stale={stale}
      acting={acting}
      deciding={deciding}
      actingAct={actingAct}
      onAnswerJudge={onAnswerJudge}
      onApproveReview={
        onApproveWave === undefined || wave.length === 0
          ? onApproveReview
          : (jobId) => onApproveWave(jobId, wave)
      }
      onRedirect={onRedirect}
    />
  );

  const region =
    board === undefined ? undefined : (
      <>
        {/* Above the plan rather than over it, the toggle on Workflow's own
            arrangement. There is no third tab: the diagram of the repository
            the same note asked for is not designed, and a disabled tab is a
            promise a screen cannot keep. */}
        <div className="armada-plan-tab__modes">
          <Tabs
            items={PLAN_VIEWS.map((one) => ({ id: one, label: PLAN_VIEW_LABEL[one] }))}
            value={view}
            onChange={(id) => onView(id as PlanView)}
          />
        </div>
        {view === "graph" ? (
          <div className="armada-plan-tab__graph">
            <WorkflowCanvas
              nodes={graph.nodes}
              edges={graph.edges}
              label={`${job.title}, as the plan's groups and tasks`}
              opensOn={graph.opensOn}
            />
          </div>
        ) : (
          <PlanBoard
            {...board}
            askPending={acting}
            {...(propose === undefined ? {} : { propose })}
            {...(remove === undefined ? {} : { remove })}
            {...(move === undefined ? {} : { move })}
            {...(onAddTask === undefined
              ? {}
              : { add: { label: ADD_TASK_LABEL, onAdd: addInto, disabled: stale } })}
          />
        )}
      </>
    );

  const layers = (
    <>
      {reading === undefined ? null : (
        <PlanTaskSheet
          {...reading}
          beside={beside}
          {...(acts === undefined ? {} : { acts })}
          {...(edit === undefined ? {} : { edit })}
          {...(drop === undefined ? {} : { drop })}
          open
          floor={floor}
          {...(proposeTask === undefined ? {} : { propose: proposeTask })}
          {...(redirect === undefined ? {} : { redirect })}
          {...(peek === undefined ? {} : { drone: peek })}
          drones={taskDrones}
          {...(doing === undefined ? {} : { doing })}
          {...(lastEdit === undefined ? {} : { lastEdit })}
          {...(stop === undefined ? {} : { stop })}
          {...(patched === undefined ? {} : { patched })}
          {...(file === undefined ? {} : { file })}
          onFile={setOpenFile}
          back={toGroup ?? trail?.back}
          onClose={
            // Opened from a group's panel, Close goes back to it; jumped to
            // from another destination, back there — `trail.ts`.
            toGroup?.onBack ??
            trail?.close ??
            (() => {
              openTaskAt(null);
              setInstruction("");
            })
          }
        />
      )}
      {group === undefined ? null : (
        <PlanGroupSheet
          open
          group={group}
          {...(board === undefined ? {} : { groups: board.groups })}
          onOpenTask={(taskId) => {
            const from = group.id;
            openTaskAt(taskId);
            setFromGroup(from);
          }}
          askPending={acting}
          {...(propose === undefined ? {} : { propose })}
          {...(remove === undefined ? {} : { remove })}
          {...(move === undefined ? {} : { move })}
          {...(onAddTask === undefined
            ? {}
            : { add: { label: ADD_TASK_LABEL, onAdd: addInto, disabled: stale } })}
          floor={floor}
          under={adding !== null}
          onClose={() => setOpenGroup(null)}
        />
      )}
      {onAddTask === undefined ? null : (
        <AddTaskDialog
          open={adding !== null}
          {...(adding?.group === undefined ? {} : { group: adding.group })}
          onAddTask={(title, note) =>
            onAddTask(job.id, { title, note, scope: [], expects: "", after: adding?.after ?? "" })
          }
          onClose={() => setAdding(null)}
          {...(onSaid === undefined ? {} : { onSaid })}
        />
      )}
    </>
  );

  return { step, groups, gate, region, layers };
}

/**
 * The review whole, for a host with nowhere else to put its pieces: the gate
 * first, the plan under it, the panels after. **What Overview's gate draws
 * where the waiting step claimed a plan** — open, in place of the folded work
 * record, because the plan is the thing being reviewed.
 */
export function PlanReview(props: PlanReviewProps) {
  const { gate, region, layers } = usePlanReview(props);
  return (
    <section className="armada-plan-review" aria-label="The plan">
      {gate}
      {region}
      {layers}
    </section>
  );
}

/**
 * Why the redirect box is closed. **One Drone per Job today**, so a task on a
 * Job with nothing on it has nothing to reach — `tab-workflow.tsx` says the
 * same of a step, off the same pointer.
 */
const NO_DRONE = "No Drone is on this Job, so there is nothing to redirect.";
