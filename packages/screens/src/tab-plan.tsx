// Plan — how the work is split, read before anybody approves it.
//
// **The split is the reading, not the task list.** Job 3's plan ran past a
// Judge that had refused it because nothing on screen said which tasks ran
// together, which agent each got, or what would be run once they stopped. So
// the card is the group, the boundary is under it, and what a Drone will be
// told is one press away.
//
// **Two views of one plan, Graph and List** (owner, 25 Sep 2026). The graph
// came off Workflow — `plan-canvas.ts` places it — and the list is this board.
//
// This file holds the open state of one reading. What the board says is
// `plan-board.ts` over `tab-plan-read.ts`, the lead is `plan-lead.tsx`, an ask
// is `tab-plan-ask.tsx`, and what it looks like is `PlanBoard`.

import { JudgeRefusal, PlanBoard, PlanTaskSheet, Tabs, WorkflowCanvas } from "@armada/components";
import { useState } from "react";

import type { JobDetail as JobWhole, JobSummary, StepDetail } from "@armada/protocol";

import { TAB_LABEL } from "./detail-tabs";
import { Eyebrow } from "./InsideAJob";
import { PlanLead } from "./plan-lead";
import {
  casesOf,
  criteriaOf,
  droneOfTask,
  groupsOf,
  REWRITE_ASK,
  revisionsOf,
  taskSheetOf,
  tasksOf,
  touchedByOf,
} from "./tab-plan-read";
import { planBoardOf } from "./plan-board";
import { steeringOf } from "./steering";
import { stepThatWorksTheGroups } from "./workflow-canvas";
import { PlanAskDialog, rewriteInstruction, type PlanAskInFlight } from "./tab-plan-ask";
import { planGraphOf } from "./plan-canvas";
import { PLAN_VIEWS, PLAN_VIEW_LABEL, type PlanView } from "./plan-view";
import { WavePlan } from "./wave-plan";
import { waveReadingOf, type WaveRegionProps } from "./tab-wave";
import type { HeldAct } from "./Acts";
import type { JobDraft } from "./draft/held";
import type { PlanAskKind, PlanRevisionView } from "./draft/revision";

/** The `DeclaredCheck.kind` a step recording a plan declares. `plan.ts`'s own read. */
const PLAN_RECORDED = "plan_recorded";

export type PlanTabProps = {
  job: JobSummary;
  whole: JobWhole | null;
  /**
   * The wave this Job dispatched, as the region draws it. **The same reading
   * Overview takes**, so the toggle between the graph and the list does not
   * reset on the way between the two destinations.
   */
  wave: WaveRegionProps;
  /** What this moment's boards draw that Fleet cannot serve yet. */
  draft?: JobDraft;
  /** The window is at `--window-floor`, so the inspector goes flush. */
  floor: boolean;
  /**
   * Which arrangement the plan is in, and the press that moves it. **Remembered
   * per viewer by the caller** — this package holds no storage, which is
   * `workflow-view.ts`'s own rule and not a second mechanism.
   */
  view: PlanView;
  onView: (view: PlanView) => void;
  /** Every control is refused while what is shown is not live. */
  stale: boolean;
  /** An act on this Job is already out. */
  acting: boolean;
  deciding: boolean;
  onApproveReview: (jobId: string) => void;
  onRedirect: (jobId: string, instruction: string) => void;
  /** Held, never pressed. What Drop from the wave sends, on that Job. */
  onActHeld: (act: HeldAct, jobId: string) => void;
  /**
   * Open a boundary Check's own row in the Record, by its name and the step
   * attempt that ran it. **The screen's** — `JobDetail.tsx` owns which
   * destination is open. Absent, no Check is a button.
   */
  onOpenCheck?: (name: string, stepAttempt: number) => void;
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
function planStepOf(whole: JobWhole | null): StepDetail | undefined {
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

/** What an ask came to, in the tense the answer earns. */
function answerSaid(revision: PlanRevisionView): string {
  switch (revision.answer) {
    case "asked":
      return "Asked. The Drone has not answered yet.";
    case "taken":
      return "The Drone took it, and the plan below is the one it wrote after.";
    case "refused":
      return "Refused. The plan below is the one the Drone recorded, unchanged.";
  }
}

/** What stops running if the ask stands. Nothing where nothing fell out. */
function droppedSaid(revision: PlanRevisionView): string | undefined {
  if (revision.dropped.length === 0) return undefined;
  return `It drops ${revision.dropped.map((one) => one.spec).join(", ")}.`;
}

/**
 * What the ask reached, so a refusal is read against the plan rather than
 * against the whole Job. **One task out of eight is the finding** — a person
 * looking at a refusal needs to know the other seven stand.
 */
function reachSaid(revision: PlanRevisionView, tasks: number): string | undefined {
  if (revision.task === undefined) return undefined;
  const rest = tasks - 1;
  if (rest <= 0) return `${revision.task} is the only task the ask touched.`;
  return `${revision.task} is the one task the ask touched. The other ${rest} stand as the Drone wrote them.`;
}

/**
 * Every change asked of the plan's Drone, with the answer under each.
 *
 * **The refusal is `JudgeRefusal` and not a card of its own** (`#1530`): a
 * Drone refusing a revision is the same record as a Judge refusing a step, and
 * a second shape for one record is a record that can be renamed in one place.
 */
function Revisions({
  revisions,
  tasks,
}: {
  revisions: readonly PlanRevisionView[];
  tasks: number;
}) {
  if (revisions.length === 0) return null;
  return (
    <section
      className="armada-detail-tab__region"
      aria-label="What you asked the plan's Drone to change"
    >
      <Eyebrow>What you asked the plan&apos;s Drone to change</Eyebrow>
      <ul className="armada-plan-tab__revisions">
        {revisions.map((revision) => {
          const reach = reachSaid(revision, tasks);
          const dropped = droppedSaid(revision);
          return (
            <li key={`${revision.at}-${revision.task ?? revision.group ?? revision.kind}`}>
              <p className="armada-plan-tab__asked">
                {revision.says}
                {dropped === undefined ? null : (
                  <span className="armada-plan-tab__dropped"> {dropped}</span>
                )}
              </p>
              <p className="armada-plan-tab__answered" data-answer={revision.answer}>
                {answerSaid(revision)}
              </p>
              {revision.refusal === undefined ? null : (
                <JudgeRefusal
                  {...(revision.refusal.criterion === undefined
                    ? {}
                    : { heading: `Refused — ${revision.refusal.criterion}` })}
                  finding={{
                    ...(revision.refusal.expected === undefined
                      ? {}
                      : { expected: revision.refusal.expected }),
                    ...(revision.refusal.produced === undefined
                      ? {}
                      : { produced: revision.refusal.produced }),
                    ...(revision.refusal.consequence === undefined
                      ? {}
                      : { consequence: revision.refusal.consequence }),
                  }}
                />
              )}
              {/* Outside the refusal block deliberately: `JudgeRefusal.reading`
                  hangs a tooltip about panel size off its own label, and this
                  sentence is about the plan rather than about a panel. */}
              {reach === undefined ? null : (
                <p className="armada-plan-tab__reach">{reach}</p>
              )}
            </li>
          );
        })}
      </ul>
    </section>
  );
}

export function PlanTab({
  job,
  whole,
  wave,
  draft,
  floor,
  view,
  onView,
  stale,
  acting,
  deciding,
  onApproveReview,
  onRedirect,
  onActHeld,
  onOpenCheck,
}: PlanTabProps) {
  // Which task the inspector is on. **This tab's own state, not the screen's**
  // — the sheet is contained by the destination, so a reader who leaves and
  // comes back lands on the board rather than inside one task.
  const [openTask, setOpenTask] = useState<string | null>(null);
  // The ask a person has opened and not sent. **This tab's own state too**: an
  // ask that survived leaving the destination would be a dialog opening over
  // a plan somebody has stopped reading.
  const [asking, setAsking] = useState<PlanAskInFlight | null>(null);
  // What has been typed at the open task's Drone and not sent. This tab's own
  // state, on the sheet's terms: it goes when the sheet does.
  const [instruction, setInstruction] = useState("");

  const step = planStepOf(whole);
  // Whether the split above is this Job's plan. A wave's plan is the Jobs it
  // dispatched, not a list of tasks — `plan.md` records the one and never the
  // other.
  const drawsAWave = waveReadingOf(whole, draft, wave.board) !== undefined;
  // **Only while the step that recorded the plan is waiting on a person, and
  // only while the window is live.** Past that gate the plan is a record, and
  // a stale window cannot tell whether the ask would still land.
  const revisable = step?.state === "awaiting_human" && !stale;

  const groups = groupsOf(whole, draft);
  const cases = casesOf(whole, draft);
  const touchedBy = touchedByOf(groups);
  // **The step the groups are worked at, for the failed Check's own output.**
  // The board draws what each boundary came to now that no second board does
  // (owner, 28 Sep 2026), and a Check result lives on that step's `check_runs`.
  const worksAt =
    whole === null ? undefined : whole.steps.find((one) => one.step_id === stepThatWorksTheGroups(whole));
  const board = planBoardOf(whole, draft, setOpenTask, openTask ?? undefined, revisable, worksAt, onOpenCheck);
  // The same plan, placed. **One press for one task either way** — a toggle
  // that opened a different surface from each view would be two screens.
  const graph = planGraphOf({ groups, onOpenTask: setOpenTask, openTask });
  const revisions = revisionsOf(whole, draft, step);
  const reading = openTask === null ? undefined : taskSheetOf(openTask, groups, cases, touchedBy);
  const rewrite =
    reading === undefined || !revisable
      ? undefined
      : {
          ...REWRITE_ASK,
          pending: acting,
          onAsk: (instruction: string) =>
            onRedirect(job.id, rewriteInstruction(reading.id, instruction)),
        };
  // **Telling this task's own Drone something, in the surface the task is read
  // in** — review and reply are one loop (`#1536`). Drawn only past the gate: a
  // plan still waiting on a person offers the rewrite ask above instead, and two
  // boxes about one task would be two ways to say the same thing to nobody.
  const open = openTask === null ? undefined : tasksOf(groups).find((one) => one.id === openTask);
  const drone = open === undefined || revisable ? undefined : droneOfTask(whole, open);
  const steering = steeringOf(job, whole);
  const redirect =
    reading === undefined || drone === undefined
      ? undefined
      : {
          reaches: drone.label,
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

  return (
    /* The sheet is the panel's sibling and not its child: the panel is the box
       that scrolls, and a layer positioned inside it slides out of the window
       with the reading — the defect `.armada-screen__detail` already carries
       the whole argument for. */
    <>
    <div className="armada-detail-tab" role="tabpanel" aria-label={TAB_LABEL.plan}>
      {/* Why the plan was formed, first — the owner's call of 28 Sep 2026.
          Everything under it is how the work was split to meet it. */}
      <PlanLead
        criteria={criteriaOf(whole, draft)}
        job={job}
        step={step}
        stale={stale}
        acting={acting}
        deciding={deciding}
        onApproveReview={onApproveReview}
        onRedirect={onRedirect}
      />
      {/* What the split became, above the plan that drew it: the wave is what
          a person came to this destination to read on a Job that dispatched
          one, and the task board underneath is how it was written. #1544. */}
      <WavePlan
        {...wave}
        {...(step === undefined ? {} : { planStep: step })}
        floor={floor}
        onDropFromWave={(jobId) => onActHeld("kill_job", jobId)}
      />
      {board === undefined ? (
        /* **A Job whose plan is a wave has recorded one**, and the split above
           is it — so the task board's own absence is not "no plan yet", which
           read as a contradiction under a drawn wave. #1544. */
        drawsAWave ? null : (
          <p className="armada-inside__absent" role="note">
            {step === undefined
              ? "This workflow records no plan, so there is no split to read."
              : `No plan yet — ${step.label} records it.`}
          </p>
        )
      ) : (
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
              onAsk={(group, ask) => setAsking({ group, ask: ask as PlanAskKind })}
            />
          )}
        </>
      )}
      {/* Above the criteria and the gate, because a refusal is the answer to
          the last thing a person did and the gate is the next thing they will
          do. */}
      <Revisions revisions={revisions} tasks={groups.flatMap((one) => one.tasks).length} />
    </div>
    {reading === undefined ? null : (
      <PlanTaskSheet
        {...reading}
        open
        floor={floor}
        {...(rewrite === undefined ? {} : { rewrite })}
        {...(redirect === undefined ? {} : { redirect })}
        onClose={() => {
          setOpenTask(null);
          setInstruction("");
        }}
      />
    )}
    <PlanAskDialog
      groups={groups}
      inFlight={asking}
      onCancel={() => setAsking(null)}
      onSend={(instruction) => {
        setAsking(null);
        onRedirect(job.id, instruction);
      }}
    />
    </>
  );
}

/**
 * Why the redirect box is closed. **One Drone per Job today**, so a task on a
 * Job with nothing on it has nothing to reach — `tab-workflow.tsx` says the
 * same of a step, off the same pointer.
 */
const NO_DRONE = "No Drone is on this Job, so there is nothing to redirect.";
