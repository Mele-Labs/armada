// Workflow — the Job's run, drawn as the workflow it froze. `#1539`.
//
// **The steps, and nothing of the plan on the canvas** (owner, 25, 28 and 29
// Sep 2026). The whole plan hung off the step that wrote it until 25 Sep, then
// one Plan node did, and on 29 Sep that node went too (`nm0h`): the Workflow
// board draws none, and the step that makes or works the plan says so in its
// own panel, with a link to the Plan tab. The second edge went with the group
// nodes, knowingly: `plan-canvas.ts` carries what was traded away. A task is
// opened from Plan; `docs/journeys/monitor-active-work.md` carries what the
// retired implement board still owes.
//
// **Canvas by default, stacked available, at every width** (#1530). Narrow
// opens on where you are: a whole plan fitted into 768px is cards nobody can
// read, so the canvas narrows onto the step a person is on rather than
// shrinking the run. The toggle is remembered per viewer, and where it is kept
// is the caller's: this package holds no storage.

import {
  DRONE_ACTIVITY,
  RunTreeSkeleton,
  Tabs,
  Tooltip,
  WorkflowCanvas,
  WorkflowInspector,
  WorkflowStacked,
  type RunTreeSkeletonProps,
} from "@armada/components";
import { useEffect, useState } from "react";
import type { Diff, JobDetail as JobWhole, JobSummary, JudgeAnswer } from "@armada/protocol";

import type { ConfirmableAct, HeldAct } from "./Acts";
import type { ActAnswer, ActingAct } from "./pending";
import type { Render } from "./render";
import { StepActs } from "./StepActs";
import { refusedOn } from "./refused-on";
import { TAB_LABEL } from "./detail-tabs";
import { isWorking, type DroneView } from "./draft/drone";
import { taskGroupsOf, type GroupView } from "./draft/group";
import { elapsedSince } from "@armada/screens/src/duration";
import { droneLabelOf, droneSays } from "./tab-drones-read";
import { NEEDS_YOU, type HeldCommand } from "./drone-held";
import { waitingOf } from "./step";
import type { TrailProps } from "./trail";
import { STEP_STOP } from "@armada/screens/src/copy";
import { steeringOf } from "./steering";
import { ordered } from "@armada/screens/src/facts";
import { placeOf, stepNodeId, stepThatWorksTheGroups, workflowRunOf } from "./workflow-canvas";
import { spentOf, workflowReadingOf } from "./workflow-inspector";
import { JudgeAsked, judgeAskedOn } from "./judge-asked";
import { heldByAFlag } from "./gaming";
import { checksAgain } from "./gates";
import { GamingHeld } from "./gaming-held";
import type { Opens } from "./phases";
import { AddedSheets, useAddedSteps, withAddedSteps } from "./added-steps";
import { WORKFLOW_VIEWS, WORKFLOW_VIEW_LABEL, type WorkflowView } from "./workflow-view";
import { pulseViewOf } from "./draft/pulse";
import { holdingOf, lookOf } from "./mine";
import { pulseCard } from "./tab-overview";
import type { Examination, Holds } from "@armada/protocol";

export type WorkflowTabProps = {
  /**
   * What Pulse reads, so the panel of the step at work draws its figures —
   * the Overview's Pulse card's, folded there (the owner, 4 Oct 2026). The
   * panel asks for the read while it is open on that step, as Pulse does.
   * Absent draws none.
   */
  pulse?: { resources: Holds; examination: Examination; onNeedPulse: (jobId: string | null) => void };
  job: JobSummary;
  /** The Job whole. `null` while the read is in flight, or where it failed. */
  whole: JobWhole | null;
  /** Why there is no run to draw, where there is none. Absent draws nothing. */
  absent?: string;
  /**
   * The run as far as it is known while this Job's own read is out — the
   * workflow's step names, from `whileReading`. **Present only until the read
   * answers**, and it wins over `absent`, whose sentence is for a read that did.
   */
  reading?: RunTreeSkeletonProps;
  /**
   * Whether the window is narrow enough that the inspector takes the whole
   * width of the tab when it opens, rather than a panel's width over one side
   * of the canvas. `useNarrow`'s answer.
   */
  narrow: boolean;
  view: WorkflowView;
  onView: (view: WorkflowView) => void;
  /**
   * The plan's groups, for what the working step counts and for what the
   * inspector says about the step that works them (`#1532`'s draft, on
   * `JobDetailProps.draft`). **Absent derives one group per task from what
   * Fleet serves**, which is thinner and never broken — `draft/group.ts`
   * carries the reasoning.
   */
  groups?: readonly GroupView[];
  /** Every Drone the Job has had, as the Drones tab lists them. */
  drones: readonly DroneView[];
  /** The command a Drone is held on, where one is — drawn on that Drone's row. */
  heldCommand?: HeldCommand | undefined;
  /** Every control is refused while what is shown is not live. */
  stale: boolean;
  /** A press is out and Fleet has not answered. */
  acting: boolean;
  actingAct?: ActingAct;
  /**
   * Answer a Judge's refusal on the open step — Overview's own handler, for
   * Overview's own block, drawn first in the step's panel (`judge-asked.tsx`).
   */
  onAnswerJudge: (jobId: string, askedAt: string, answer: JudgeAnswer, note?: string) => void;
  /**
   * A step the gaming check holds, answered in its panel with Overview's own
   * block and handlers (`gaming-held.tsx`): the patch its hunks come out of,
   * how a flag's brief opens, Carry on and Send it back.
   */
  diff: Diff;
  /** Ask for this Job's patch, or let it go with `null`. */
  onReadDiff: (jobId: string | null) => void;
  opens: Opens;
  onOverrule: (jobId: string, reason: string) => void;
  onSendBack: (jobId: string, note?: string) => void;
  onRedirect: (jobId: string, instruction: string) => void;
  onAct: (act: ConfirmableAct, jobId: string) => void;
  onActHeld: (act: HeldAct, jobId: string) => void;
  /**
   * What a stopped step's panel offers, as Overview's lead does: `StepActs`,
   * off Fleet's `stuck.recourse`, with the same handlers.
   */
  render: Render;
  rerunningChecks: boolean;
  answered?: ActAnswer | undefined;
  onRerun: (jobId: string) => void;
  onRerunChecks: (jobId: string) => void;
  /**
   * Where the plan link in a step's panel goes: the Plan tab. **The screen's,
   * not this tab's** — `JobDetail.tsx` owns which destination is open, and a tab that moved it
   * itself would be a second place the strip can be driven from.
   */
  onOpenPlan: () => void;
  /**
   * Opens a Drone the step panel lists, in the Drones tab with its sheet open
   * (owner, 29 Sep 2026: a jump with a way back, not a second panel beside
   * this one). **The screen's**, as `onOpenPlan` is. Absent draws the rows as
   * facts rather than presses.
   */
  onOpenDrone?: (droneId: string) => void;
  /**
   * The way back after a jump into this tab, and where this tab's open step
   * is reported so a jump out of it can return — `trail.ts`.
   */
  trail?: TrailProps;
  /**
   * The step to land on with its panel open — a step pressed in the Record's
   * reading. Read once, when the tab opens; after that the panel is the
   * person's.
   */
  opensStep?: string;
};

export function WorkflowTab({
  job,
  whole,
  absent,
  reading: unread,
  narrow,
  view,
  onView,
  groups: given,
  drones,
  heldCommand,
  stale,
  acting,
  actingAct,
  onAnswerJudge,
  diff,
  onReadDiff,
  opens,
  onOverrule,
  onSendBack,
  onRedirect,
  onAct,
  onActHeld,
  render,
  rerunningChecks,
  answered,
  onRerun,
  onRerunChecks,
  onOpenPlan,
  onOpenDrone,
  trail,
  opensStep,
  pulse,
}: WorkflowTabProps) {
  // The node a person has open. **Not the running step held in state** — that
  // moves under them as the Job advances, and a panel that changed subject
  // while somebody was reading it is the surface this screen exists to escape.
  const [open, setOpen] = useState<string | null>(
    opensStep === undefined ? null : stepNodeId(opensStep),
  );
  // **And again whenever it changes**, not only at mount: Back after a jump
  // out of this tab lands on the step it left, and the screen hands that step
  // in here (`trail.ts`). Plan answers its own `opensTask` the same way.
  useEffect(() => {
    if (opensStep !== undefined) setOpen(stepNodeId(opensStep));
  }, [opensStep]);
  // Whether the canvas re-centres on the running step as the Job advances.
  // **Off until it is asked for**: it wins over the fit, and a run opened
  // centred on one card is a run with its other steps off screen.
  const [following, setFollowing] = useState(false);
  const openStep = setOpen;
  const added = useAddedSteps(job.id, job.workflow_id);
  // Which step is open, told to the trail, so a jump out of it can come back
  // here with the same step open. Its id is the step's, which is what
  // `opensStep` lands on.
  const openedStep = whole?.steps.find((step) => stepNodeId(step.step_id) === open);
  useEffect(() => {
    trail?.onHere(openedStep === undefined ? null : { id: openedStep.step_id, label: openedStep.label });
    // `trail` is rebuilt by the screen every render; what matters is the step.
  }, [openedStep?.step_id]);
  // **The patch, while a held step's panel is open**, so the block draws the
  // flagged lines as Overview's does. Overview reads it for as long as it is
  // mounted and lets it go when it is not, and the citation alone is what the
  // panel drew in its place.
  const flagHeld = heldByAFlag(whole, openedStep);
  // The step at work, open: its panel draws Pulse's figures, and asks for them
  // while it is open, as the Pulse tab does — and lets the read go after.
  const atWork =
    openedStep !== undefined && openedStep.state === "running" && openedStep.step_id === whole?.job.current_step_id;
  useEffect(() => {
    if (!atWork || pulse === undefined) return;
    pulse.onNeedPulse(job.id);
    return () => pulse.onNeedPulse(null);
  }, [atWork, job.id]);
  const holding = pulse === undefined ? null : holdingOf(pulse.resources, job.id);
  const looked = pulse === undefined ? undefined : lookOf(pulse.examination, job.id);
  const figures =
    !atWork || pulse === undefined
      ? undefined
      : pulseCard(holding === null ? null : pulseViewOf(holding), looked?.state === "found" ? looked.examined : null, whole);
  useEffect(() => {
    if (!flagHeld) return;
    onReadDiff(job.id);
    return () => onReadDiff(null);
  }, [flagHeld, job.id]);

  // **Nothing scrolls the panel into view any more, because it is never out of
  // it.** Until 25 Sep the panel was a column that folded under the whole graph
  // at a narrow window, so a press on a task moved a reply box nobody could
  // see and an effect wrote scroll position to fix it. The panel is a layer
  // over the canvas now and sticks to the top of the destination
  // (`screens.css`), so review and reply stay one loop with no scrolling at all.

  const toggle = (
    <Tabs
      items={WORKFLOW_VIEWS.map((one) => ({ id: one, label: WORKFLOW_VIEW_LABEL[one] }))}
      value={view}
      onChange={(id) => onView(id as WorkflowView)}
    />
  );

  // **The frame the run lands in, before it has.** The toggle and the
  // workflow's name are already in hand; the canvas is drawn empty at its own
  // height, and the stacked run names its steps with a bar where each will say
  // where it stands.
  if (whole === null && unread !== undefined) {
    return (
      <div className="armada-detail-tab" role="tabpanel" aria-label={TAB_LABEL.workflow}>
        <div className="armada-workflow-tab" data-view={view} data-narrow={narrow || undefined}>
          <div className="armada-workflow-tab__surface">
            <div className="armada-workflow-tab__modes">{toggle}</div>
            {view === "canvas" ? (
              <div className="armada-workflow-tab__stage">
                <div className="armada-workflow-tab__canvas" role="status" aria-label="Reading the run" aria-busy>
                  <div className="armada-workflow-tab__where">
                    <Tooltip label="The workflow this Job froze">
                      <span className="armada-workflow-tab__workflow mono">{job.workflow_id}</span>
                    </Tooltip>
                  </div>
                </div>
              </div>
            ) : (
              <RunTreeSkeleton {...unread} />
            )}
          </div>
        </div>
      </div>
    );
  }

  if (whole === null || whole.steps.length === 0) {
    return (
      <div className="armada-detail-tab" role="tabpanel" aria-label={TAB_LABEL.workflow}>
        {absent === undefined ? null : (
          <p className="armada-inside__absent" role="note">
            {absent}
          </p>
        )}
      </div>
    );
  }

  const groups = given ?? taskGroupsOf(whole);
  const groupsUnder = stepThatWorksTheGroups(whole);
  // The steps. A press on one opens it in the panel.
  const plain = workflowRunOf({
    whole,
    groups,
    drones,
    selected: open,
    onOpen: openStep,
    ...(heldCommand === undefined ? {} : { held: heldCommand }),
  });
  const run = withAddedSteps(added, whole, plain);
  // **Nothing is open until a press opens it** (owner, 25 Sep 2026) — here, or
  // on the step's name in the Record's reading, which lands with it open. The panel
  // used to land on the step the Job is on, so the column beside the canvas was
  // never blank — there is no column now. The canvas has the tab's whole width
  // and this is a layer over it, so a reading nobody asked for would be a panel
  // covering the run it exists to explain.
  const reading = workflowReadingOf({ whole, groups, selected: open, groupsUnder, onOpenPlan, drones });
  const steering = steeringOf(job, whole);
  const label = `${job.title}, as its workflow's run`;
  // The board's corner: which workflow this is, and which step the Job is on.
  // **No step count** — the steps are drawn right under it.
  const on = ordered(whole).find((step) => step.step_id === whole.job.current_step_id);

  // Every Drone that worked the step that is open, running or not (owner, 29
  // Sep 2026), running first, then in task order. A press opens one in the
  // Drones tab, with a way back here. **The Drones tab's own list**, so the two
  // never disagree about who worked a step.
  const now = Date.now();
  const openStepId = whole.steps.find((step) => stepNodeId(step.step_id) === open)?.step_id;
  const here = drones
    .filter((one) => one.step === openStepId)
    .sort((a, b) => Number(isWorking(b)) - Number(isWorking(a)));
  const held = (one: DroneView): boolean => heldCommand?.droneId === one.id;
  const ranFor = (one: DroneView): string | undefined =>
    one.ended_at !== undefined
      ? elapsedSince(one.since, one.ended_at)
      : one.state === "running"
        ? elapsedSince(one.since, now)
        : undefined;

  // Nothing until a press, and then **the app's own panel**: the floating
  // Sheet Record, Drones and Plan draw theirs in, over the work area and
  // dimming it (owner, 29 and 30 Sep 2026), with the way back in its head after
  // a jump here. So a second step is read by closing this one first.
  // **The step Fleet's stop names, and nothing already asking on it.** A
  // question or a held flag carries its own answers in `asks`; otherwise the
  // panel says why it stopped and offers what Overview's lead does. **Nothing
  // while its Checks run again**, as the lead offers nothing: Fleet withholds
  // its recourse for that span, and the track in the header says what is on.
  const stoppedHere =
    render === "stopped" &&
    openedStep !== undefined &&
    whole.stuck?.step_id === openedStep.step_id &&
    !checksAgain(openedStep) &&
    !judgeAskedOn(whole, openedStep) &&
    !flagHeld;
  const why = stoppedHere ? refusedOn(openedStep, whole.acceptance_criteria) : undefined;
  const judgeAsk = judgeAskedOn(whole, openedStep) ? (
    <JudgeAsked
      jobId={job.id}
      whole={whole}
      step={openedStep}
      stale={stale}
      acting={acting}
      actingAct={actingAct}
      onAnswerJudge={onAnswerJudge}
    />
  ) : flagHeld ? (
    // Overview's own block for a step the gaming check holds, with the same
    // handlers (owner, 2 Oct 2026, #1672).
    <GamingHeld
      job={job}
      whole={whole}
      step={openedStep}
      diff={diff}
      opens={opens}
      stale={stale}
      acting={acting}
      actingAct={actingAct}
      onOverrule={onOverrule}
      onSendBack={onSendBack}
      onRedirect={onRedirect}
    />
  ) : undefined;
  // **A command held on this step with no one Drone to put it on** — the wire
  // names the step and not the Drone — is the step's to ask, where several
  // work at once. One Drone at work takes it on its own row instead.
  const stepHeld =
    heldCommand !== undefined && heldCommand.droneId === undefined && heldCommand.stepId === openedStep?.step_id
      ? heldCommand.node
      : undefined;
  const asks = waitingOf(judgeAsk, stepHeld);
  const layer =
    reading === undefined ? null : (
      <WorkflowInspector
        {...reading}
        {...(figures === undefined ? {} : { pulse: figures })}
        {...(!stoppedHere
          ? {}
          : {
              stopped: {
                ...(why === undefined ? {} : { why }),
                acts: (
                  <StepActs
                    job={job}
                    whole={whole}
                    opens={opens}
                    render={render}
                    acting={acting}
                    actingAct={actingAct}
                    answered={answered}
                    rerunningChecks={rerunningChecks}
                    stale={stale}
                    onAct={onAct}
                    onRedirect={onRedirect}
                    onOverrule={onOverrule}
                    onRerun={onRerun}
                    onRerunChecks={onRerunChecks}
                  />
                ),
              },
            })}
        {...(asks === undefined ? {} : { asks })}
        sheet={{ back: trail?.back }}
        // After a jump here, Close goes back, as Plan's, Drones' and Record's
        // do (owner, 30 Sep 2026); otherwise it closes the step.
        onClose={trail?.close ?? (() => openStep(null))}
        // The redirect box went (owner, 29 Sep 2026, `losq`): a person never
        // knows which Drone to message. The step's Drones take its place.
        running={{
          rows: here.map((one) => ({
            id: one.id,
            label: droneLabelOf(one, whole),
            // The Drones tab's own mark for the state, named on hover — a
            // mark, never a word (owner, 2 Oct 2026).
            activity: held(one) ? "awaiting_human" : DRONE_ACTIVITY[one.state],
            said: held(one) ? NEEDS_YOU : droneSays(one),
            ...(held(one) ? { asking: heldCommand?.node } : {}),
            says: [one.task, ranFor(one), ...spentOf(one)].filter((part) => part !== undefined).join(" · "),
          })),
          ...(onOpenDrone === undefined ? {} : { onOpen: onOpenDrone }),
        }}
        {...(steering.act === undefined
          ? {}
          : {
              stop: {
                children: STEP_STOP.label,
                askLabel: STEP_STOP.ask,
                description: STEP_STOP.said,
                disabled: acting && actingAct !== "kill_drone",
                pending: acting && actingAct === "kill_drone",
                onAsk: () => onAct("kill_drone", job.id),
                onCommit: () => onActHeld("kill_drone", job.id),
              },
            })}
      />
    );

  return (
    <div className="armada-detail-tab" role="tabpanel" aria-label={TAB_LABEL.workflow}>
      {/* The run is a spine and nothing under it. The canvas takes the height
          the destination has left, as the board draws it, and never less than
          `--h-workflow-canvas`. */}
      <div className="armada-workflow-tab" data-view={view} data-narrow={narrow || undefined}>
        <div className="armada-workflow-tab__surface">
          {/* Above the run rather than over it: drawn inside the canvas the
              toggle sat on top of the last step's card at every width.

              No `?` here any more. Guide 11 explained how a group gets its
              second edge; the plan's graph moved to the Plan tab on 25
              September 2026, so the guide was retired and its number with it. */}
          <div className="armada-workflow-tab__modes">{toggle}</div>
          {view === "canvas" ? (
            <div className="armada-workflow-tab__stage">
              <div className="armada-workflow-tab__canvas">
                <div className="armada-workflow-tab__where">
                  <Tooltip label="The workflow this Job froze">
                    <span className="armada-workflow-tab__workflow mono">{job.workflow_id}</span>
                  </Tooltip>
                  {on === undefined ? null : (
                    <>
                      <span className="armada-workflow-tab__rule" aria-hidden="true" />
                      <span className="armada-workflow-tab__on">step {placeOf(on)}</span>
                    </>
                  )}
                </div>
                <WorkflowCanvas
                  nodes={run.nodes}
                  edges={run.edges}
                  label={label}
                  running={run.running}
                  following={following}
                  onFollowing={setFollowing}
                  opensOn={run.opensOn}
                  hangsFromTop
                  runsDown
                  reveals={added.open}
                />
              </div>
            </div>
          ) : (
            <WorkflowStacked label={label} rows={run.rows} />
          )}
        </div>

        {layer}
        <AddedSheets added={added} whole={whole} />
      </div>
    </div>
  );
}
