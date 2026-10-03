// The Overview tab — today's arrangement, and every reading it is built from.
//
// This file holds the open state of one reading: which step, which sheet, where
// the log was held. What any region says, it does not decide — the question box
// is `step.tsx`, the sheets are `Sheets.tsx`, and the board itself is
// `OverviewBoard.tsx`.
//
// **The Job header and the tab strip belong to `JobDetail.tsx`.** An addition
// to a region goes in that region's file; an addition to the screen goes there.

import {
  Button,
  CPU_USAGE,
  GROUP_STATE,
  MEMORY_USAGE,
  processesTotal,
  worktreesTotal,
} from "@armada/components";
import { useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";

import type { FollowedLog, JobDetail as JobWhole } from "@armada/protocol";
import { heldForMoney, heldForTurns } from "./Acts";
import { useCheckOutputs, useFollowing } from "./outputs";

/** What `followed` reads as where the caller hands none in. */
const NOT_FOLLOWING: FollowedLog = { state: "none" };
import { useFrames } from "./frames";
import { openArtifact, openPullRequest } from "./opening";
import { planOf } from "./plan";
import { declaredAgainstTouched, editsIn, filesByTask } from "./task-files";
import { useDetailKeys } from "./detail-keys";
import { DetailSheet, type OpenSheet, type SheetMove, type SheetReading } from "./Sheets";
import type { JobCheckLog } from "./check-log-sheet";
import { leadOf } from "./lead";
import { heldByAFlag } from "./gaming";
import { GamingHeld } from "./gaming-held";
import { OverviewBoard } from "./OverviewBoard";
import type { DetailTab } from "./detail-tabs";
import type { CheckAt } from "./tab-record";

import { shapeSaid } from "./plan-board";

import { workflowRunOf } from "./workflow-canvas";
import type { Figure } from "@armada/components";
import type { JobExamined } from "@armada/protocol";
import type { PulseView } from "./draft/pulse";
import type { DroneView } from "./draft/drone";
import type { LandingRule } from "./draft/landing";
import { changedOf } from "./settings";
import { LandBoard } from "./LandBoard";
import { TrainBoard } from "./TrainBoard";
import { landedOf } from "./landed";
import { span } from "./duration";
import { ordered } from "./facts";
import { holdingOf, logOf, lookOf, turnsOf } from "./mine";
import { verdictSlotOf } from "./verdict-answered";
import { useDiffAgain } from "./produced";
import { useRunSheet } from "./rehearsal";
// Which Check's output `o` opens.
import { outputOf } from "./gates";
import { whyNoSteps } from "./run";
import { answeringOf, commandOf, questionOf, waitingOf } from "./step";
import { proposerWaitOf } from "./proposing";
import { taskGroupsOf } from "./draft/group";
import {
  LOOK_FAILED,
  nothingToAsk,
  pulseFiguresOf,
  pulseReadingOf,
  whyNoReading,
} from "./resources";
import { pulseViewOf } from "./draft/pulse";
import { jobMembersOf } from "./draft/members";
import { membersOf, useDroppedMembers } from "./members";
import { waveReadingOf } from "./tab-wave";
import { stillReading, whyNoBrief } from "./work";
import { whyUnreachable } from "./while-reading";



import type { JobDetailProps } from "./detail-props";
import type { Render } from "./render";
import type { TrailProps } from "./trail";
import { FIRST_PLAN_VIEW } from "./plan-view";

/**
 * What the screen hands its first tab: everything the caller handed it, plus
 * the four things `JobDetail` worked out once for the header and the strip.
 *
 * **Reconciled here rather than twice.** `job` is `whole.job` where Fleet has
 * answered and the Board's row until then — read in one place so the header
 * and the run cannot disagree about which record they are drawing.
 */
export type OverviewTabProps = JobDetailProps & {
  whole: JobWhole | null;
  render: Render;
  /** Whether the window is under `--layout-breakpoint`, so the inspector folds. */
  narrow: boolean;
  /** Whether the window is at `--window-floor`. */
  floor: boolean;
  /**
   * The sheet layer's own state, held by the screen rather than by this tab —
   * the Job header opens Settings, and a header belongs to no destination.
   */
  onSheet: SheetReading;
  onMove: (move: SheetMove) => void;
  /** The three dialogs the Job header owns and this tab's keys open. */
  onReporting: (open: boolean) => void;
  onRaising: (open: boolean) => void;
  onRaisingTurns: (open: boolean) => void;
  /** A card's press. Overview is a door to each destination — the owner, 29 Sep. */
  onOpenTab: (tab: DetailTab) => void;
  /**
   * Open the Record on one Check's row. **The screen's, not this tab's** —
   * `JobDetail.tsx` owns which destination is open, exactly as it does for the
   * Plan's group boundary.
   */
  onOpenCheck: (at: CheckAt) => void;
  /** Open a boundary Check's log in the log panel. **The screen's**, which holds the panel. */
  onOpenCheckLog: (log: JobCheckLog) => void;
  /**
   * The Job header's own acts, drawn again as the lead's where the lead's act
   * is approving the dispatch. **The same element, not a second control**, so
   * the press, the face while it is out and the menu behind the caret cannot
   * drift apart from the header's.
   */
  headerActs: ReactNode;
  /**
   * What the approval approves, `approving.tsx`, drawn under the lead while it
   * offers the approval. **The screen's**, which holds what a person moved on
   * it until the header's press sends it.
   */
  approval?: ReactNode;
  /**
   * What a plan at the review gate takes that only the screen holds: a Drone
   * opened from a task's peek, the task to land on, and the way back —
   * `PlanTab`'s own three. Absent, the peek draws no Open.
   */
  onOpenDrone?: (droneId: string) => void;
  opensTask?: string;
  trail?: TrailProps;
  /** Every Drone the Job has had, which a task's panel lists its own from. */
  drones?: readonly DroneView[];
};

/**
 * What `Decide` is handed for the claims' read: nothing. This tab holds the read
 * for every review gate, because which gate draws depends on the claims it
 * would fetch — so `Decide` neither opens nor closes it.
 */
const HELD_ABOVE = () => {};

/**
 * The Settings card's one line: where the work lands, and whether anybody has
 * moved a setting on this Job since it was approved.
 *
 * **`changedOf` is the count the tab strip already carries**, so the card and
 * the tab cannot say different numbers.
 */
function settingsSaid(landing: LandingRule | undefined, changed: number): string {
  const lands = landing?.target == null ? "Lands where the Manifest says" : `Lands in ${landing.target}`;
  const moved =
    changed === 0 ? "nothing changed since" : `${changed} ${changed === 1 ? "change" : "changes"} since`;
  return `${lands} · ${moved}`;
}

/**
 * What Overview's Pulse card holds: what is alive on this Job, then what it is
 * taking from the machine. The owner's call of 29 Sep 2026, against the card
 * that read one worktree size and nothing else.
 *
 * **The destination's own first three, then totals rather than counts.** The
 * cost group is cut, because Spend and Turns are the lead's own and a card
 * repeating them is the same figure twice on one screen; `pulseFiguresOf`
 * marks that group's first figure `apart`, which is where the cut falls — the
 * boundary is the band's own, not a list of labels typed here.
 *
 * `processesTotal` is `null` while nothing is running and `worktreesTotal` is
 * `""` where any worktree went unmeasured; neither draws a row.
 */
function pulseCard(view: PulseView | null, examined: JobExamined | null, whole: JobWhole | null): Figure[] {
  const band = pulseFiguresOf(view, whole);
  const cost = band.findIndex((figure) => figure.apart === true);
  const alive = cost === -1 ? band : band.slice(0, cost);
  if (view === null) return alive;
  const reading = pulseReadingOf(view, examined);
  const running = processesTotal(reading.processes);
  const disk = worktreesTotal(reading.worktrees);
  const taking: Figure[] = [
    ...(running === null
      ? []
      : [
          { label: CPU_USAGE, value: running.cpu },
          { label: MEMORY_USAGE, value: running.memory },
        ]),
    ...(disk === "" ? [] : [{ label: "Worktree", value: disk }]),
  ];
  const [first, ...rest] = taking;
  return first === undefined ? alive : [...alive, { ...first, apart: true }, ...rest];
}

export function OverviewTab(props: OverviewTabProps) {
  const {
    onReadDiff,
    onOpenArtifact,
    onOpenPullRequest,
    onReadCheckOutput,
    followed,
    onFollowCheckOutput,
    onReadFrame,
    onFrameSrc,
    onNeedPulse,
    whole,
    render,
    watched,
    manifests,
    stale,
    now,
    acting,
    actingAct,
    onAnswer,
    onAnswerCommand,
    onExplainCommand,
    observed,
    resources,
    examination,
    onExamine,
    recorded,
    onRedirect,
    onAnswerJudge,
    onCopied,
    onSaid,
    rehearsal,
    onCompose,
    onReporting,
    onRaising,
    onRaisingTurns,
  } = props;
  const job = props.job;
  // Which step the panel is showing. **The whole of navigation inside a Job**:
  // `null` means the one Fleet says is current, so a Job that moves on carries
  // the reader with it until they choose a step themselves.
  const [selected, setSelected] = useState<string | null>(null);

  // Which sheet is up and what it is reading. Held by the screen, so the Job
  // header can open one: `Sheets.tsx`'s `SheetReading`.
  const onSheet = props.onSheet;
  const move = props.onMove;
  // Whether the folded inspector is up. Read only while `narrow`: above the
  // bound the inspector is a column and this decides nothing.
  const [, setInspecting] = useState(false);
  // Which members somebody dropped in this window. Nothing serves a drop, so
  // what a press changes is what is drawn — `useDroppedMembers` says why.
  const dropping = useDroppedMembers();
  const sheet = onSheet.which;
  // Which Check the check-output sheet is open on, or `undefined` for none —
  // #1021. `checkSheetOf` in `Sheets.tsx` is what turns this into live or kept.
  const openCheckId = onSheet.which === "check" ? onSheet.checkId : undefined;

  // Select a step. **Clicking the running step resumes following it, the same
  // way clicking anything else holds there** — `job.current_step_id` is the
  // one value a click is compared against, so a Job that later advances onto
  // whatever the reader is holding still reads as following it, and only a
  // click on the step that is running now clears the hold. #1152.
  function selectStep(stepId: string): void {
    setSelected(stepId === job.current_step_id ? null : stepId);
    setInspecting(true);
  }



  const runHook = useRunSheet({ ...rehearsal, jobId: job.id, jobTitle: job.title, sheet, now, setSheet: (which) => move({ move: "open", which }), onSaid });

  // The diff, for every Job that is open rather than only for one at review.
  // **A produced file opens to what it actually wrote**, and it did that on one
  // status because the review block was the only thing asking for the read. It
  // is still the expensive read, so the diff sheet and the review decision draw
  // from this one answer.
  useEffect(() => {
    onReadDiff(job.id);
    return () => {
      onReadDiff(null);
    };
  }, [job.id]);

  // Whether the inspector has a column of its own, and whether the sheet it
  // folds to goes flush. Both read from tokens rather than from a media query,
  // which cannot see one — `floor.ts` carries the whole of why.
  const floor = props.floor;

  const manifest = manifests.find((held) => held.id === job.owner_manifest_id);
  // The half of a board's reading no operation answers yet — `draft/held.ts`.
  // Absent on a real Fleet, and each board falls back to the wire.
  const draft = props.draft ?? {};
  // Every reading, narrowed to the Job on screen. **All five carry the id they
  // were taken for and all five are checked against it** — each lags a
  // selection by a round trip, so another Job's answer landing here would be
  // that Job's steps under this Job's title, its turns under this Job's step,
  // its disk under this Job's panel. `mine.ts` is the one place the check is
  // written, and the counterpart to `main/reader.ts` on this side of the seam.
  // (`whole` itself was read above, before `job` was reconciled to it.)
  // What the Board's own row already answers, while this Job's read is out.
  const watching = turnsOf(observed, job.id);
  const noted = logOf(props.journalled, job.id);
  // The open sheet's own reading of the patch, taken again as the Job writes.
  useDiffAgain(onReadDiff, job.id, sheet, watching?.rows ?? [], job.assigned_drone !== undefined);
  const holding = holdingOf(resources, job.id);
  const looked = lookOf(examination, job.id);
  // The finding itself, or none. Read twice — the summary asks whether an
  // absence is a fault, the sheet draws every look — so it is named once.
  const examinedNow = looked?.state === "found" ? looked.examined : null;

  const steps = whole === null ? [] : ordered(whole);
  const open = steps.find((step) => step.step_id === (selected ?? job.current_step_id)) ?? steps[0];
  // The workflow overview, while this Job waits for approval — what will run,
  // in place of the idle step view `open` above would otherwise draw. #1149.
  // What a Job that has finished shows, above the run that finished it. #1542.
  const landed = landedOf({ job, whole, draft, manifest, holding });

  // The strip's rows carry the three records a person reads because a verdict
  // went against them, and each opens. The Job id and the toast are the panel's,
  // so they are handed down rather than reached for; `phases.tsx` says why.
  // What this step's Drone claimed it did, and the documents the Judge tier
  // points at. Read off the same `evidence` the verdict slot is built from
  // rather than fetched again — one call, drawn once. A read that has not
  // arrived is not a step that claimed nothing.
  const claimed = useMemo(
    () =>
      recorded.evidence.state === "read"
        ? recorded.evidence.steps.find((one) => one.step_id === open?.step_id)
        : undefined,
    [recorded.evidence, open?.step_id],
  );
  // **The claims are held open by this tab for as long as a gate is drawn,
  // not by `Decide`.** The gate chooses what it draws off `claimed` — a plan
  // is Plan's review, which has no `Decide` in it — so a read opened and
  // closed by `Decide` closed itself the moment it said "plan", and the gate
  // flipped between the two for as long as the Job was open.
  const needMaterial = props.onNeedMaterial;
  useEffect(() => {
    if (render !== "reviewing") return;
    needMaterial(job.id);
    return () => needMaterial(null);
  }, [job.id, render]);
  const opensRecords = useMemo(
    () => ({ jobId: job.id, open: onOpenArtifact, onSaid }),
    [job.id, onSaid],
  );
  // The detail's contextual tier, and the open state it moves. Bound while a
  // Job is open and not before, so nothing on the Board listens for a key that
  // means nothing there — and the press is swallowed only where something
  // answered it.
  useDetailKeys({
    // `f`, from `actions.toml` — `open_diff`, scope `detail`. It opens the
    // layer now rather than a chapter: the patch stopped being something the
    // panel draws.
    onOpenSheet: () => openSheet("diff"),
    onOpenRun: () => runHook.open(), // `r`, Journey 9 — freed from `review`
    // `o` — a Check's output, from the open step. The failed Check first: a
    // person reaching for an output on a step that stopped wants the one that
    // says why, and on a step where nothing failed the key opens the first
    // output there is rather than nothing.
    onOpenOutput: () => {
      const kept = outputOf(open);
      if (kept === undefined) return;
      void openArtifact(onOpenArtifact, job.id, { kept, what: "check" }).then((because) => {
        if (because !== null) onSaid(because);
      });
    },
    // `b`, on the one render that offers the act. Elsewhere the shape carries
    // nothing and the press is left alone rather than answered with a dialog
    // the header is not offering.
    ...(render === "stopped" && !stale ? { onReport: () => onReporting(true) } : {}),
    // `B`, on the one state that offers the act: a job stopped for money.
    // Elsewhere the shape carries nothing and the press is left alone rather
    // than answered with a dialog no control on this screen is offering.
    ...(heldForMoney(job) && !stale ? { onRaiseCap: () => onRaising(true) } : {}),
    // `T`, on the other ceiling. The two are exclusive because `budget_hold`
    // is, so a job held for turns answers this key and not `B`.
    ...(heldForTurns(job) && !stale ? { onRaiseTurnCap: () => onRaisingTurns(true) } : {}),
    // `n`, scope `anywhere` — the Board's own key. Detail answers it with the
    // same handler the caller wired the Board's row to.
    onCompose,
  });

  // What each Check printed, for as long as this Job is open. **Held for the
  // Job rather than for a step**, because a Job's steps each have their own
  // Checks and a reader moving between them should not re-fetch a file it
  // already has.
  const outputs = useCheckOutputs(onReadCheckOutput, job.id);
  // The running Check's log somebody is reading, for as long as this Job is
  // open. Streamed by main, and let go when the Job changes.
  const following = useFollowing(onFollowCheckOutput, followed ?? NOT_FOLLOWING, job.id);

  // The frames each step captured, for as long as this Job is open. Held for
  // the Job for `outputs`' reason, and it owns the object URLs it mints — a
  // `blob:` lives until it is revoked, so a person walking six Jobs would
  // otherwise leave six sets of screenshots behind them.
  const frames = useFrames(onReadFrame, job.id, onFrameSrc);

  // **Asked for when the step is opened, not when a chapter is pressed.** The
  // claim #209 makes is that a change whose point is not the code is reviewed
  // by looking at it, and a panel that draws a button saying there are pictures
  // has not led with anything. `want` is idempotent per `kept`, so this asking
  // again on every tick of the clock sends nothing twice.
  const shownBy = open?.frames;
  useEffect(() => {
    if (shownBy !== undefined && shownBy.length > 0) frames.want(shownBy);
  }, [shownBy, frames]);

  const turns = watching === null ? [] : watching.rows;

  /** Open a sheet. **The second one replaces the first** rather than stacking on it. */
  function openSheet(which: Exclude<OpenSheet, null | "check" | "task">): void {
    move({ move: "open", which });
  }

  /** Close it. */
  function closeSheet(): void {
    move({ move: "close" });
  }

  // The verdict sheet's slot: `Decide`'s place at the gate, and the finished
  // Job's own place, whichever of the three arrangements the render is —
  // `verdictSlotOf`, in `verdict-answered.tsx`.
  //
  // **Under the lead, with the Drone's question and the command.** The owner's
  // call of 29 Sep 2026, on a screen whose lead read *Delivery is waiting on
  // you to approve it* over a button that reached nothing: an approval is
  // rank 1 in *Overview leads with what releases the most*, and rank 1 is
  // answered where it is said. The diff and the pull request's comments stay
  // where they were — Record reads the Job, this decides it.
  const verdictSlot = verdictSlotOf({
    job,
    whole,
    open,
    render,
    recorded,
    opensRecords,
    now,
    claimed,
    frames,
    // Held above, so `Decide` neither opens nor closes it.
    onNeedMaterial: HELD_ABOVE,
    onNeedRemarks: props.onNeedRemarks,
    stale,
    acting,
    actingAct,
    answered: props.answered,
    deciding: props.deciding,
    decidingAct: props.decidingAct,
    onMergePullRequest: props.onMergePullRequest,
    onRerunFailedChecks: props.onRerunFailedChecks,
    onInvestigateFailedChecks: props.onInvestigateFailedChecks,
    onQueueAfterFinding: props.onQueueAfterFinding,
    onFileFindingIssue: props.onFileFindingIssue,
    onOpenFindingIssue: props.onOpenFindingIssue,
    onApproveReview: props.onApproveReview,
    onRequestChanges: props.onRequestChanges,
    onReject: props.onReject,
    onTakeUpRemarks: props.onTakeUpRemarks,
    onOpenRemarkLink: props.onOpenRemarkLink,
    onOpenPullRequest,
    onSaid,
    onAnswerJudge,
    onOpenDiff: () => openSheet("diff"),
    onDismissFinding: props.onDismissFinding,
    notes: noted?.notes ?? [],
    // A plan at this gate is Plan's own review, off the same props the Plan
    // destination hands it — and the same Graph/List choice, so the plan reads
    // one way in both. The patch's read is this tab's already.
    plan: {
      job,
      whole,
      ...(props.draft === undefined ? {} : { draft: props.draft }),
      floor,
      view: props.planView ?? FIRST_PLAN_VIEW,
      onView: (view) => props.onPlanView?.(view),
      stale,
      acting,
      deciding: props.deciding,
      actingAct,
      onApproveReview: props.onApproveReview,
      onAnswerJudge,
      ...(props.onApproveWave === undefined ? {} : { onApproveWave: props.onApproveWave }),
      board: props.board ?? [],
      onRedirect,
      ...(props.onTaskAct === undefined ? {} : { onTaskAct: props.onTaskAct }),
      ...(props.onMovePlan === undefined ? {} : { onMovePlan: props.onMovePlan }),
      models: props.models?.models ?? [],
      onAddTask: props.onAddTask,
      onDropTask: props.onDropTask,
      onSaid,
      diff: recorded.diff,
      ...(props.opensTask === undefined ? {} : { opensTask: props.opensTask }),
      ...(props.onOpenDrone === undefined ? {} : { onOpenDrone: props.onOpenDrone }),
      ...(props.drones === undefined ? {} : { drones: props.drones }),
      now,
      onOpenCheckLog: props.onOpenCheckLog,
      ...(props.trail === undefined ? {} : { trail: props.trail }),
    },
  });


  // Read once: `plan` gates both the region and its own eyebrow act, and a
  // second call would be a second, possibly different, reading of `whole`.
  // **A wave's plan is the split above, not a list of tasks.** `plan.md` is
  // what the epic workflow's plan step records, so the task well's own "no
  // plan yet" placeholder sat under a drawn wave and contradicted it. #1544.
  const drawsAWave = waveReadingOf(whole, props.draft, props.board ?? []) !== undefined;
  const planRead = planOf(whole);
  const plan = drawsAWave && planRead?.recorded !== true ? undefined : planRead;
  // The Jobs landing under this one. The mock hands the whole reading; against
  // a real Fleet it is derived from the Board's own rows, which carry each
  // member's status, its branch and whether its pull request merged.
  //
  // **Not for a Job that dispatched a wave** (`#1544`). The two regions answer
  // different questions: members are a landing order — each one parked until
  // the one before it lands — and a wave is a dependency graph whose Jobs land
  // whenever each is done. Drawn together, this band said "parked until the
  // one before it lands" about a Job that had already merged, and the same
  // five Jobs were drawn twice on one screen. The derivation from the Board's
  // rows cannot tell the two apart, because `dispatched_by` is all it has.
  const memberDraft = props.draft?.members;
  const membersRead = drawsAWave
    ? undefined
    : (memberDraft ?? (whole === null ? undefined : jobMembersOf(whole, props.board ?? [])));
  const members = membersOf(dropping.over(membersRead), props.draft?.landing, {
    ...(props.onOpenJob === undefined ? {} : { onOpenJob: props.onOpenJob }),
    // The member's own Job id, never the address the card drew — the header's
    // own rule, in `opening.ts`.
    onOpenPullRequest: (member) => {
      void openPullRequest(onOpenPullRequest, member).then((because) => {
        if (because !== null) onSaid(because);
      });
    },
    onAnswerJudge,
    // **Offered only on a reading the mock handed in.** No operation closes a
    // pull request or rebases what was stacked on a branch, so against a real
    // Fleet this would be a press that does nothing. `#1543`.
    ...(memberDraft === undefined ? {} : { onDropMember: dropping.drop }),
    stale,
    acting,
  });
  // `#1432`. What the open task said it would touch against what it did, from
  // the Job's own turns rather than the step's — a task's files include what
  // it wrote on an earlier run, `grouped.tsx`'s rule for the same reading.
  // Built here because the inputs are this screen's: `Sheets` holds neither.
  const taskTouched =
    onSheet.which !== "task" || whole?.work_plan === undefined
      ? undefined
      : declaredAgainstTouched(
          whole.work_plan.tasks.find((task) => task.id === onSheet.taskId)?.scope ?? [],
          filesByTask(
            editsIn(turns, whole.work_plan.tasks),
            // The patch where it has been read. A task's own edits name their
            // paths either way; the diff is what turns a call's path into the
            // repository's, so without it the paths are the calls' own.
            recorded.diff.state === "read" ? (recorded.diff.work?.files ?? []) : [],
          ).get(onSheet.taskId) ?? [],
        );

  // The sheet layer, which the header opens on a Job at any state — so it is
  // built beside the arrangement rather than inside it.
  const sheetSlot =
        open === undefined ? null : (
          <DetailSheet
            which={sheet}
            job={job}
            whole={whole}
            step={open}
            diff={recorded.diff}
            checkId={openCheckId}
            {...(onSheet.which === "task" ? { taskId: onSheet.taskId } : {})}
            {...(taskTouched === undefined ? {} : { taskTouched })}
            planTasks={plan?.recorded === true ? plan.tasks : undefined}
            outputs={outputs}
            following={following}
            // The Pulse board, derived exactly as the Pulse tab derives it —
            // `Refresh` came with it, because it acts on this reading and not
            // on the five lines that open it. Two derivations of one `Held`
            // would let the sheet and the tab disagree about the same Job.
            holds={{
              jobId: job.handle,
              reading: holding === null ? null : pulseReadingOf(pulseViewOf(holding), examinedNow, undefined, now),
              figures: pulseFiguresOf(holding === null ? null : pulseViewOf(holding), whole),
              note: whyNoReading(resources),
              age: holding === null ? undefined : (span(holding.read_at, now) ?? undefined),
              examined: examinedNow,
              looking: looked?.state === "looking",
              lookFailed: looked?.state === "failed" ? LOOK_FAILED : undefined,
              nothingToAsk: nothingToAsk(resources),
              onExamine: () => onExamine(job.id),
            }}
            // The sheet draws the same board as the Pulse tab, so it holds the
            // same poll open while it is the sheet on screen. #1571.
            onNeedPulse={onNeedPulse}
            run={runHook.slot}
            floor={floor}
            onClose={closeSheet}
          />
        );

  // **What the lead is about, under the lead.** A Drone's question and its
  // answers, and the Allow / Always allow / Reject choice for a command it was
  // not given. These were the step panel's until 29 Sep 2026; the reframe took
  // the panel off Overview and took them with it, so `Answer it` and
  // `Decide it` named acts with nowhere to happen. The owner put them back
  // here rather than at a destination: the lead says a thing is waiting, and
  // the thing waiting is what a person came to answer.
  const answering = answeringOf(job.id, stale, acting, onAnswerCommand, actingAct);
  // Bound to the Job being read, so nothing downstream carries an id back.
  const explain =
    onExplainCommand === undefined ? undefined : (call: string) => onExplainCommand(job.id, call);
  // The step Fleet's own record of the stop names, which is the one a flag holds.
  const heldStep = whole?.steps.find((one) => one.step_id === whole.stuck?.step_id);
  const waiting = waitingOf(
    questionOf(whole, job.id, stale, acting, onAnswer, actingAct),
    commandOf(whole, answering, explain),
    // A step the gaming check holds, answered where the lead names it — the
    // block the Workflow step panel draws too (owner, 2 Oct 2026, #1672).
    // Only where a flag holds it: an element is a slot drawn, even one that
    // renders nothing, and a drawn slot takes the lead's act away.
    !heldByAFlag(whole, heldStep) ? undefined : (
      <GamingHeld
        job={job}
        whole={whole}
        step={heldStep}
        diff={recorded.diff}
        opens={opensRecords}
        stale={stale}
        acting={acting}
        actingAct={actingAct}
        onOverrule={props.onOverrule}
        onSendBack={props.onSendBack}
        onRedirect={onRedirect}
        underTheLead
      />
    ),
    verdictSlot,
    // The proposer's own call, on the one status where the thing the lead names
    // is a model reading rather than anything a step holds. #1159.
    proposerWaitOf(job, props.proposing ?? null, now, props.onStopProposer),
  );

  // **What leads the screen, before the run a person would have to read to
  // find it.** The owner's 29 September call: Overview shows what needs you,
  // and it leads with one thing. `lead.ts` holds the rank and its costs.
  const lead = leadOf(job, whole, now);
  // **No act where the thing to act on is already under the lead.** A button
  // named `Decide it` over the Allow / Reject choice it scrolls to is a press
  // that moves nothing.
  //
  // **The act goes where the thing it names actually lives**, which since the
  // reframe took the step panel off Overview is another destination. Until 29
  // Sep 2026 every one of them selected a step instead, and nothing has read
  // that selection since `InsideAJob` was deleted — four buttons, four dead
  // presses, which is what the owner pressed and reported.
  //
  // **The approval is the header's own control, drawn here too**, rather than
  // a route to the proposal: that route reached `ProposalTab`, which draws only
  // off a draft no real Fleet serves, so on the owner's Job of 1 Oct 2026 the
  // lead said *Waiting for your approval* and offered nothing.
  const opens = lead.opens;
  const fixJob = lead.fix?.job;
  const openJob = props.onOpenJob;
  const openFix = fixJob === undefined || openJob === undefined ? undefined : () => openJob(fixJob);
  const leadAct =
    lead.act === undefined || waiting !== undefined ? undefined : lead.approves === true ? (
      props.headerActs
    ) : open === undefined ? undefined : (
      <Button
        onClick={() => {
          if (opens === undefined) {
            // Selecting the step is what opens the inspector on it, which is
            // where the question box and the gate's own acts already are.
            selectStep(open.step_id);
            return;
          }
          if ("check" in opens) props.onOpenCheck(opens.check);
          else props.onOpenTab(opens.tab);
        }}
      >
        {lead.act}
      </Button>
    );

  // **The lead, a strip, and a card per destination.** The arrangement
  // `InsideAJob` drew here — the run tree, the plan well, the pointers and the
  // step inspector — was the Job's detail rather than its state, which is what
  // the owner opened a failing Job and could not see past on 29 Sep 2026.
  const groups = props.draft?.groups ?? (whole === null ? [] : taskGroupsOf(whole));
  const canvas = whole === null || whole.steps.length === 0 ? undefined : workflowRunOf({ whole, groups });
  const tasks = (whole?.work_plan?.tasks ?? []).filter((task) => task.state !== "dropped");

  const inside = (
    <OverviewBoard
      // The quiet line stands in while the read is out; any other lead is
      // the Board row's to say at once (owner, 1 Oct 2026).
      lead={{
        ...lead,
        act: leadAct,
        reading: lead.quiet === true && stillReading(watched, job.id),
        // The fix's title opens that Job, where the shell can open one. #1673.
        ...(lead.fix === undefined
          ? {}
          : { fix: { ...lead.fix, ...(openFix === undefined ? {} : { onOpen: openFix }) } }),
        // Each Job parked on this one's fix, by the wire's title or the
        // Board's, and a press that opens it. #1673.
        ...(lead.parkedOnIt === undefined
          ? {}
          : {
              parked: lead.parkedOnIt.map((one) => ({
                job: one.job,
                title: one.title ?? props.board?.find((row) => row.id === one.job)?.title ?? one.job,
                ...(openJob === undefined ? {} : { onOpen: () => openJob(one.job) }),
              })),
            }),
      }}
      waiting={waiting}
      // **What the approval approves, only while the lead offers it.** The
      // owner approved Job 1 on 1 Oct 2026 without seeing what counted as
      // done or how its steps gate, and the Judge refused the plan for it.
      {...(lead.approves !== true || waiting !== undefined || props.approval === undefined
        ? {}
        : { approving: props.approval })}
      {...(canvas === undefined
        ? { workflowAbsent: whyNoSteps(watched, job.id) }
        : {
            workflow: {
              nodes: canvas.nodes,
              edges: canvas.edges,
              label: `${job.title}, as its workflow's run`,
              opensOn: canvas.opensOn,
            },
          })}
      {...(tasks.length === 0
        ? {
            // **A refused read says so**, as Brief and Workflow do: "No plan
            // has been recorded" there would answer a question nobody read.
            planAbsent:
              whyUnreachable(watched, job.id) ??
              (plan?.recorded === false ? `${plan.stepLabel} has not recorded one yet.` : undefined),
          }
        : {
            plan: {
              working: tasks
                .filter((task) => task.state === "working")
                .map((task) => ({ id: task.id, title: task.title })),
              groups: groups.map((group) => ({
                ordinal: group.ordinal,
                tasks: `${group.tasks.length} ${group.tasks.length === 1 ? "task" : "tasks"}`,
                // **How it runs is drawn, not named** — the owner's sketch of
                // 29 Sep 2026, after `2 tasks, one after another` and then a
                // `sequential` chip both said in words what a reader pictures
                // anyway. `shapeSaid` is Plan's own sentence, and it is what
                // the drawing says to a reader who cannot see it.
                count: group.tasks.length,
                concurrent: group.concurrent,
                shapeLabel: shapeSaid(group),
                state: group.state,
                said: GROUP_STATE[group.state]?.verb ?? group.state,
              })),
            },
          })}
      // **What this machine is carrying, not what the Job has spent.** Spend
      // and Turns are the strip's, two regions up; a card repeating them is
      // the same figure twice on one screen.
      //
      // Totals rather than counts: the owner replaced Pulse's process count
      // with CPU and memory on 29 Sep 2026 — *"I would prefer to use this
      // space for total CPU/memory instead of a number of processes."*
      // **A Job being proposed is on no machine at all**, so `0 Checks running`
      // would be a measurement of something that cannot exist yet rather than a
      // reading of nothing. The card says nothing was read instead.
      pulse={
        job.status === "proposing"
          ? []
          : pulseCard(holding === null ? null : pulseViewOf(holding), examinedNow, whole)
      }
      pulseAbsent={whyNoReading(resources)}
      {...(whole === null ? {} : { brief: whole.facts })}
      briefAbsent={whyNoBrief(watched, job.id)}
      {...(whole?.from_studio === undefined || props.onOpenStudio === undefined
        ? {}
        : { fromStudio: whole.from_studio, onOpenStudio: props.onOpenStudio })}
      // What froze, and whether anybody has moved a setting since. `changedOf`
      // is the same count the strip's own Settings tab carries.
      settings={settingsSaid(props.draft?.landing, changedOf(whole))}
      reading={stillReading(watched, job.id)}
      onOpenTab={props.onOpenTab}
    />
  );


  // A Job whose members are Jobs is read for the order and the one decision
  // open on it — `TrainBoard`, which takes this destination the way the Land
  // board does. The parent's own run is the Workflow tab's.
  if (members !== undefined) {
    return (
      <>
        <TrainBoard read={members} onSaid={onSaid} onCopied={onCopied} />
        {sheetSlot}
      </>
    );
  }

  // A Job that has finished is read for what it came to, not for the run that
  // is over: the board takes this destination and the run is the Workflow
  // tab's, the plan the Plan tab's, the ledger the Record tab's. #1542.
  // **The sheet layer goes with every arrangement, not only the two boards.**
  // It was dropped from this return when `InsideAJob` came off, and `r`, `L`
  // and `f` each opened nothing on an ordinary Job — the keys were still
  // bound, and there was no layer for what they opened to be drawn in.
  if (landed === undefined)
    return (
      <>
        {inside}
        {sheetSlot}
      </>
    );
  return (
    <>
      <LandBoard
        read={landed}
        onOpenPullRequest={() => {
          void openPullRequest(onOpenPullRequest, job.id).then((because) => {
            if (because !== null) onSaid(because);
          });
        }}
        onCompose={onCompose}
        onCopied={onCopied}
      />
      {sheetSlot}
    </>
  );
}