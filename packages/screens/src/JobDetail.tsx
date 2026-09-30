// One Job, read whole, at seven destinations: Overview, Workflow, Plan, Record,
// Drones, Pulse, Settings. Overview is the arrangement this screen has always had — the run as a
// tree, the selected step in the inspector, its story in the order it happened
// — and the other four are where the readings that used to compete for that one
// panel go instead. `#1534`.
//
// **This file is the screen, not a tab.** It reads the Job whole, draws the
// header, the standing callout and the strip, and hands each tab what it needs.
// What a tab holds is that tab's own module: `tab-overview.tsx`,
// `tab-workflow.tsx`, `tab-plan.tsx`, `tab-record.tsx`, `tab-drones.tsx`,
// `tab-pulse.tsx`, `tab-settings.tsx`.

import { JobDetailHeaderActions, type JobResourcesProps } from "@armada/components";
import { useCallback, useEffect, useReducer, useRef, useState } from "react";
import { useAtFloor, useNarrow } from "@armada/shell";

import { countsOf, FIRST_TAB, JobTabs, type DetailTab } from "./detail-tabs";
import { headingOf, Unrenderable } from "./heading";
import { detailOf } from "./mine";
import { renderFor } from "./render";
import { replacedCallout } from "./replaced";
import { holdingOf, lookOf } from "./mine";
import { span } from "./duration";
import {
  LOOK_FAILED,
  nothingToAsk,
  pulseFiguresOf,
  pulseReadingOf,
  whyNoReading,
  type CapPresses,
} from "./resources";
import { pulseViewOf } from "./draft/pulse";
import { NO_SHEET, sheetMoved } from "./Sheets";
import type { JobDetail as JobWhole } from "@armada/protocol";
import { openArtifact } from "./opening";
import { OverviewTab } from "./tab-overview";
import { ProposalTab } from "./tab-proposal";
import { FrozenAtApproval } from "./frozen-at-approval";
import { proposalEditsOf } from "./tab-proposal-read";
import { DronesTab } from "./tab-drones";
import { PlanTab } from "./tab-plan";
import { PulseTab } from "./tab-pulse";
import { RecordTab, type CheckAt } from "./tab-record";
import { SettingsTab } from "./tab-settings";
import { whyNothingToChange } from "./settings";
import { WorkflowTab } from "./tab-workflow";
import { type WaveRegionProps } from "./tab-wave";
import { WavePlan } from "./wave-plan";
import { whyNoSteps } from "./run";
import { FIRST_PLAN_VIEW } from "./plan-view";
import { FIRST_WORKFLOW_VIEW } from "./workflow-view";
import { ledgerOf } from "./draft/ledger";
import { useTrail } from "./trail";

export type { ConfirmableAct, HeldAct, JobAct } from "./Acts";
export type { FoldedReads } from "./mine";
export { renderFor } from "./render";
export type { Render } from "./render";
export type { DetailTab } from "./detail-tabs";
export { DETAIL_TABS, TAB_LABEL } from "./detail-tabs";

/**
 * What a caller hands this screen. It lives in `detail-props.ts`, moved there
 * when this file crossed the 900-line refusal for the third time.
 *
 * **Re-exported here on purpose.** A caller names the screen it is
 * configuring, not the file the type sits in.
 */
export type { JobDetailProps } from "./detail-props";
import type { JobDetailProps } from "./detail-props";

/**
 * The screen, **remounted for every Job it is handed.** Everything below holds
 * the open state of one reading, and none of it is the next Job's — so the
 * reset is the key, rather than an effect per piece that lands a frame late and
 * a piece nobody wrote one for.
 *
 * **Where things are is the one exception**, and it is not held here at all:
 * `whereOpen` is Fleet's own preference, handed in and saved by the caller —
 * this package stays free of Electron. That is what survives a Job switch
 * and a relaunch alike, on its own.
 */
export function JobDetail(props: JobDetailProps) {
  return <OneJob key={props.job.id} {...props} />;
}

function OneJob(props: JobDetailProps) {
  // Which destination is open. **Not held across Jobs**: a reader who opened
  // Pulse on a wedged Job is not asking for Pulse on the next one, and the key
  // above resets it with everything else.
  const [tab, setTab] = useState<DetailTab>(FIRST_TAB);
  // The step Workflow opens on, where the Record's or the Drones' reading sent
  // a person there. Cleared by the strip, so the next visit opens on nothing.
  const [opensStep, setOpensStep] = useState<string | undefined>(undefined);
  // The task Plan opens on, where the Drones' reading sent a person there.
  // Cleared by the strip, on `opensStep`'s terms.
  const [opensTask, setOpensTask] = useState<string | undefined>(undefined);
  // The Check whose Record row opens, where the Plan's boundary sent a person
  // there. Cleared by the strip in the same way.
  const [opensCheck, setOpensCheck] = useState<CheckAt | undefined>(undefined);
  // Whether the lead's approval act asked for the proposal. Cleared by the
  // strip, on `opensStep`'s terms — the proposal is Overview's, not a tab.
  const [opensProposal, setOpensProposal] = useState(false);
  const [opensDrone, setOpensDrone] = useState<string | undefined>(undefined);
  const [opensRow, setOpensRow] = useState<string | undefined>(undefined);
  // The way back across a jump between destinations — `trail.ts`.
  const trail = useTrail((to) => {
    setOpensTask(to.tab === "plan" || to.tab === "overview" ? to.open?.id : undefined);
    setOpensDrone(to.tab === "drones" ? to.open?.id : undefined);
    setOpensRow(to.tab === "record" ? to.open?.id : undefined);
    setOpensCheck(undefined);
    setOpensProposal(false);
    setTab(to.tab);
  });
  const toTab = (next: DetailTab) => {
    trail.clear();
    setOpensRow(undefined);
    setOpensStep(undefined);
    setOpensTask(undefined);
    setOpensDrone(undefined);
    setOpensCheck(undefined);
    setOpensProposal(false);
    setTab(next);
  };

  // The cap a press on Pulse's Spend or Turns went to Settings for, until
  // Settings has drawn and its row is in view. **Found by `data-ceiling`**
  // after the switch commits, because the row does not exist before it.
  const [reaching, setReaching] = useState<"cost" | "turns" | null>(null);
  const screen = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (tab !== "settings" || reaching === null) return;
    const row = screen.current?.querySelector<HTMLElement>(`[data-ceiling="${reaching}"]`);
    row?.scrollIntoView({ block: "nearest" });
    // Raise is the row's one control, and what a person came to press.
    row?.querySelector<HTMLElement>("button")?.focus({ preventScroll: true });
    setReaching(null);
  }, [tab, reaching]);

  // Whether the report dialog is up. Two controls open it — the Job header's
  // menu entry and `b` — and the keyboard is bound on the tab that draws the run.
  const [reporting, setReporting] = useState(false);
  // Whether the raise dialog is up, on `reporting`'s terms: the header's
  // button and `B` both open it.
  const [raising, setRaising] = useState(false);
  // Whether the turn-cap dialog is up. Its own state beside the cost cap's:
  // `budget_hold` offers one control or the other, never both.
  const [raisingTurns, setRaisingTurns] = useState(false);

  // What a person has moved on the proposal, before anything is approved.
  // **Held by the screen rather than by the tab**, so reading the Record and
  // coming back does not throw away a retyped title — nothing here has been
  // sent, so this window is the only place it exists.
  const [edits, setEdits] = useState(() => proposalEditsOf(props.draft));

  // Which sheet is up and what it is reading — `Sheets.tsx`'s `SheetReading`.
  // **Held here and not on the tab**, because the header opens one: Settings
  // is a menu entry on a header that no destination owns. Overview is what
  // draws the layer, so opening one from the header lands there.
  const [onSheet, move] = useReducer(sheetMoved, NO_SHEET);

  // The Job whole, read for the id the prop carries — `mine.ts`'s own check,
  // taken this early because what it answers is which Job this binding reads.
  const whole = detailOf(props.watched, props.job.id);
  // **The one binding every tab reads.** `whole.job` is Fleet's own answer to
  // `GET /jobs/:job_id`, fetched and re-read for this exact Job, so once it has
  // arrived it stands in for the board row the prop carries, which can lag a
  // `job.state_changed` event that missed or has not yet applied.
  const job = whole?.job ?? props.job;
  // Read once: the proposal and the frozen reading both draw a gate that
  // defers to this repository's policies.
  const manifest = props.manifests.find((one) => one.id === job.owner_manifest_id);
  const render = renderFor(job);

  // Whether the inspector has a column of its own, and whether a folded sheet
  // goes flush. Both are tokens read off the document — a media feature value
  // cannot be a custom property, and `floor.ts` carries the whole of why.
  const narrow = useNarrow();
  const floor = useAtFloor();

  // Why the Workflow tab has no run to draw, where it has none. The same
  // reading Overview's run column takes, so the two never give a Job's empty
  // workflow two different reasons.
  const absent = whyNoSteps(props.watched, props.job.id);

  // Stable across a tick of `now`, which is what keeps the wave's canvas from
  // rebuilding its nodes every second.
  const opens = props.onOpenJob;
  const openJob = useCallback((jobId: string) => opens?.(jobId), [opens]);

  const wave: WaveRegionProps = {
    job,
    whole,
    ...(props.draft === undefined ? {} : { draft: props.draft }),
    board: props.board ?? [],
    questions: props.questions ?? [],
    repositories: props.repositories ?? [],
    now: props.now,
    stale: props.stale,
    acting: props.acting,
    // Graph or List, **Plan's own remembered choice**: on an Epic Job the wave
    // is what Plan draws, and Overview draws the same wave, so one toggle
    // reads it one way everywhere (owner, 30 Sep 2026).
    view: props.planView ?? FIRST_PLAN_VIEW,
    onView: (view) => props.onPlanView?.(view),
    // A wave whose caller offers no way to open a Job draws its cards inert
    // rather than pressing into nothing — `onOpenJob` is the shell's, and
    // optional for that reason.
    onOpenJob: openJob,
    onAnswerJudge: props.onAnswerJudge,
    onAnswerCommand: props.onAnswerCommand,
  };

  // The Job header, and everything that goes in it. `heading.tsx` holds what
  // it is made of — the badge, the facts, the acts that end or replace the Job,
  // and the way out to the pull request — which is where the next thing added
  // to this header goes rather than here.
  const heading = headingOf({
    job,
    whole,
    now: props.now,
    render,
    stale: props.stale,
    acting: props.acting,
    actingAct: props.actingAct,
    answered: props.answered,
    approving: props.approving,
    reporting,
    onReporting: setReporting,
    onAct: props.onAct,
    onActHeld: props.onActHeld,
    onApprove: props.onApprove,
    onReport: props.onReport,
    onRaiseCap: props.onRaiseCap,
    raising,
    onRaising: setRaising,
    onRaiseTurnCap: props.onRaiseTurnCap,
    raisingTurns,
    onRaisingTurns: setRaisingTurns,
    onOpenPullRequest: props.onOpenPullRequest,
    onOpenJob: props.onOpenJob,
    onCopied: props.onCopied,
    onSaid: props.onSaid,
  });

  // The badge is the header, so a Job the registry has no glyph or verb for
  // cannot be drawn at all — which is the `null` above. Named rather than
  // half-drawn.
  if (heading === null || render === "unrenderable") {
    return <Unrenderable job={job} />;
  }

  // Spend and Turns press through to their caps only where Settings draws
  // caps to change; a Job that is over gets its note there instead.
  const reach = (cap: "cost" | "turns") => {
    setReaching(cap);
    setTab("settings");
  };
  const caps: CapPresses | undefined =
    whyNothingToChange(job, whole) === undefined
      ? { cost: () => reach("cost"), turns: () => reach("turns") }
      : undefined;

  return (
    <div className="armada-screen__detail" ref={screen}>
      <JobDetailHeaderActions {...heading} onCopied={props.onCopied} />
      {/* Under the header and above the strip, because a job that was replaced
          is where a person lands and no one destination can say so. #1439. */}
      {replacedCallout(whole?.replaced_by, props.onOpenJob)}
      <JobTabs value={tab} onChange={toTab} counts={countsOf(whole, job)} />

      {tab === "overview" && edits !== undefined && (edits.proposal.approved_at === undefined || opensProposal) ? (
        // A Job at its approval gate: the proposal is what Overview has to
        // draw, because no step has run and approving it is the one thing
        // waiting. **And no wave**: a Job not approved has dispatched nothing.
        //
        // **Only until it is approved, or until the lead asks for it.** After
        // the press the frozen values are a reading that never changes and
        // Settings holds them — the owner, 29 Sep 2026; a Job sent back to the
        // gate is read here again because that is where it is answered.
        <ProposalTab
          job={job}
          whole={whole}
          edits={edits}
          onEdits={setEdits}
          models={props.models?.models ?? []}
          workflows={props.workflows}
          stale={props.stale}
          manifest={manifest}
        />
      ) : tab === "overview" ? (
        // **One box scrolls the wave and the board under it**, as Plan's does:
        // with the wave outside the board's own scroller, the graph took the
        // height and the board was squeezed to nothing — nothing under the
        // wave could be reached.
        <div className="armada-detail-tab armada-overview-scroll">
        {/* The wave this Job dispatched, above the run — what it dispatched is
            the product of an Epic Job, and the run is how it got there. A Job
            that dispatched nothing draws nothing. #1544. A Job pressed opens
            its panel, with what it asks of you at the top — Plan's own. */}
        <WavePlan {...wave} floor={floor} onDropFromWave={(jobId) => props.onActHeld("kill_job", jobId)} />
        <OverviewTab
          {...props}
          job={job}
          whole={whole}
          render={render}
          narrow={narrow}
          floor={floor}
          onSheet={onSheet}
          onMove={move}
          onReporting={setReporting}
          onRaising={setRaising}
          onRaisingTurns={setRaisingTurns}
          onOpenTab={setTab}
          // The lead's act, where what it names is a Check that failed: the
          // Record, on that Check's row. **The Plan boundary's own route**,
          // below — one way into a Check's row, pressed from two places.
          onOpenCheck={(at) => {
            setOpensCheck(at);
            setTab("record");
          }}
          // The lead's approval act. **Handed down only where there is a
          // proposal to draw**, so a Job at the gate with no proposal read
          // offers no button rather than one that reaches nothing.
          {...(edits === undefined ? {} : { onOpenProposal: () => setOpensProposal(true) })}
          {...(opensTask === undefined ? {} : { opensTask })}
          onOpenDrone={(droneId) => {
            trail.push("overview");
            setOpensDrone(droneId);
            setTab("drones");
          }}
          trail={trail.of("overview")}
        />
        </div>
      ) : tab === "workflow" ? (
        <WorkflowTab
          job={job}
          whole={whole}
          {...(absent === undefined ? {} : { absent })}
          narrow={narrow}
          view={props.workflowView ?? FIRST_WORKFLOW_VIEW}
          onView={(view) => props.onWorkflowView?.(view)}
          acting={props.acting}
          {...(props.actingAct === undefined ? {} : { actingAct: props.actingAct })}
          {...(props.draft?.groups === undefined ? {} : { groups: props.draft.groups })}
          onRedirect={props.onRedirect}
          onAct={props.onAct}
          onActHeld={props.onActHeld}
          // Where the Plan node goes. The strip is this screen's, so the run
          // asks for the destination rather than moving one itself.
          onOpenPlan={() => toTab("plan")}
          {...(opensStep === undefined ? {} : { opensStep })}
        />
      ) : tab === "plan" ? (
        <PlanTab
          job={job}
          whole={whole}
          wave={wave}
          floor={floor}
          view={props.planView ?? FIRST_PLAN_VIEW}
          onView={(view) => props.onPlanView?.(view)}
          stale={props.stale}
          acting={props.acting}
          deciding={props.deciding}
          onApproveReview={props.onApproveReview}
          {...(props.onApproveWave === undefined ? {} : { onApproveWave: props.onApproveWave })}
          board={props.board ?? []}
          onRedirect={props.onRedirect}
          onActHeld={props.onActHeld}
          diff={props.recorded.diff}
          onReadDiff={props.onReadDiff}
          {...(props.onTaskAct === undefined ? {} : { onTaskAct: props.onTaskAct })}
          {...(props.onMovePlan === undefined ? {} : { onMovePlan: props.onMovePlan })}
          models={props.models?.models ?? []}
          onAddTask={props.onAddTask}
          onDropTask={props.onDropTask}
          onSaid={props.onSaid}
          {...(props.draft === undefined ? {} : { draft: props.draft })}
          {...(opensTask === undefined ? {} : { opensTask })}
          now={props.now}
          onOpenDrone={(droneId) => {
            trail.push("plan");
            setOpensDrone(droneId);
            setTab("drones");
          }}
          onOpenCheck={(name, stepAttempt) => {
            trail.push("plan");
            setOpensCheck({ name, stepAttempt });
            setTab("record");
          }}
          trail={trail.of("plan")}
        />
      ) : tab === "settings" ? (
        <SettingsTab
          job={job}
          whole={whole}
          // What froze at approval, above the settings still open. A frozen
          // setup is settings — the owner's 29 September call.
          frozen={
            edits === undefined || edits.proposal.approved_at === undefined ? undefined : (
              <FrozenAtApproval
                landing={edits.landing}
                proposal={edits.proposal}
                whole={whole}
                manifest={manifest}
              />
            )
          }
          models={props.models ?? null}
          stale={props.stale}
          acting={props.acting}
          {...(props.actingAct === undefined ? {} : { actingAct: props.actingAct })}
          onSetWhenBlocked={props.onSetWhenBlocked}
          onSetWhenRefused={props.onSetWhenRefused}
          onSetModel={props.onSetModel}
          onSetReviewModel={props.onSetReviewModel}
          onRemoveAllowedCommand={props.onRemoveAllowedCommand}
          onRaiseCap={props.onRaiseCap}
          onRaiseTurnCap={props.onRaiseTurnCap}
        />
      ) : tab === "record" ? (
        <RecordTab
          {...recordOf(props, whole)}
          jobId={job.id}
          floor={floor}
          onReadCheckOutput={props.onReadCheckOutput}
          diff={props.recorded.diff}
          onReadDiff={props.onReadDiff}
          {...(props.draft?.groups === undefined ? {} : { groups: props.draft.groups })}
          {...(props.draft?.cases === undefined ? {} : { cases: props.draft.cases })}
          onSaid={props.onSaid}
          onOpenStep={(stepId) => {
            setOpensStep(stepId);
            setTab("workflow");
          }}
          {...(opensCheck === undefined ? {} : { opensCheck })}
          {...(opensRow === undefined ? {} : { opensRow })}
          trail={trail.of("record")}
        />
      ) : tab === "drones" ? (
        <DronesTab
          job={job}
          whole={whole}
          {...(props.draft?.drones === undefined ? {} : { drones: props.draft.drones })}
          {...(props.draft?.groups === undefined ? {} : { groups: props.draft.groups })}
          now={props.now}
          floor={floor}
          stale={props.stale}
          acting={props.acting}
          actingAct={props.actingAct}
          onRedirect={props.onRedirect}
          onAct={props.onAct}
          onActHeld={props.onActHeld}
          onOpenStep={(stepId) => {
            setOpensStep(stepId);
            setTab("workflow");
          }}
          onOpenTask={(taskId) => {
            trail.push("drones");
            setOpensTask(taskId);
            setTab("plan");
          }}
          {...(opensDrone === undefined ? {} : { opensDrone })}
          trail={trail.of("drones")}
        />
      ) : (
        <PulseTab holds={pulseOf(props, whole, job.id, caps)} jobId={job.id} onNeedPulse={props.onNeedPulse} />
      )}
    </div>
  );
}

/**
 * What the Record tab reads.
 *
 * **Composed here rather than fetched**, because no operation answers a ledger
 * yet — `draft/ledger.ts` reads the Job whole, its evidence, its footprint and
 * its history into one list, and every one of those is a read this screen is
 * already holding for another region.
 */
function recordOf(props: JobDetailProps, whole: JobWhole | null) {
  if (whole === null) return { detail: null, rows: [] };
  const jobId = props.job.id;
  const handed =
    props.recorded.handed.state === "heard" && props.recorded.handed.jobId === jobId
      ? props.recorded.handed.moment
      : undefined;
  const footprint =
    props.recorded.footprint.state === "read" && props.recorded.footprint.jobId === jobId
      ? props.recorded.footprint.reading
      : undefined;
  const evidence =
    props.recorded.evidence.state === "read" && props.recorded.evidence.jobId === jobId
      ? props.recorded.evidence.steps
      : undefined;
  const history =
    props.history?.state === "read" && props.history.jobId === jobId
      ? props.history.moves
      : undefined;
  return {
    detail: whole,
    rows: ledgerOf({
      detail: whole,
      ...(history === undefined ? {} : { history }),
      ...(evidence === undefined ? {} : { evidence }),
      ...(footprint === undefined ? {} : { footprint }),
      ...(handed === undefined ? {} : { handed }),
      ...(props.draft?.groups === undefined ? {} : { groups: props.draft.groups }),
    }),
  };
}

/**
 * The Pulse board: the machine reading, the figures over it, and the look.
 *
 * **`pulseViewOf` is the one derivation.** The board is built on the draft
 * shape (`draft/pulse.ts`), filled from what Fleet serves today — so the same
 * board draws the real Fleet and whatever `#1545` promotes onto the wire.
 */
function pulseOf(
  props: JobDetailProps,
  whole: JobWhole | null,
  jobId: string,
  caps: CapPresses | undefined,
): JobResourcesProps {
  const holding = holdingOf(props.resources, jobId);
  const view = holding === null ? null : pulseViewOf(holding);
  const looked = lookOf(props.examination, jobId);
  const examined = looked?.state === "found" ? looked.examined : null;
  const nothing = nothingToAsk(props.resources);
  return {
    reading: view === null ? null : pulseReadingOf(view, examined, whole),
    figures: pulseFiguresOf(view, whole, caps),
    note: whyNoReading(props.resources),
    ...(view === null ? {} : { age: span(view.read_at, props.now) ?? undefined }),
    examined,
    looking: looked?.state === "looking",
    ...(looked?.state === "failed" ? { lookFailed: LOOK_FAILED } : {}),
    ...(nothing === undefined ? {} : { nothingToAsk: nothing }),
    onExamine: () => props.onExamine(jobId),
    ...(props.onKillProcess === undefined
      ? {}
      : { onKillProcess: (process) => props.onKillProcess?.(jobId, process) }),
    ...(props.onKillProcesses === undefined || view === null
      ? {}
      : { onKillAll: () => props.onKillProcesses?.(jobId, view.processes.length) }),
    onOpen: (what) =>
      void openArtifact(props.onOpenArtifact, jobId, what).then((because) => {
        if (because !== null) props.onSaid(because);
      }),
  };
}
