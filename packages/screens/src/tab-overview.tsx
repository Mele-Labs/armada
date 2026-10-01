// The Overview tab — today's arrangement, and every reading it is built from.
//
// This file holds the open state of one reading: which step, which sheet, where
// the log was held. What any region says, it does not decide — the step's
// facts, band and question box are `step.tsx`, the story is `chapters.tsx`, the
// sheets are `Sheets.tsx`, and the arrangement itself is `InsideAJob.tsx`.
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

import type { FollowedLog, JobDetail as JobWhole } from "@armada/protocol";
import { heldForMoney, heldForTurns } from "./Acts";
import { useCallArguments } from "./calls";
import { useCheckOutputs, useFollowing } from "./outputs";

/** What `followed` reads as where the caller hands none in. */
const NOT_FOLLOWING: FollowedLog = { state: "none" };
import { useFrames } from "./frames";
import { openArtifact, openPullRequest } from "./opening";
import { planOf } from "./plan";
import { declaredAgainstTouched, editsIn, filesByTask } from "./task-files";
import { DIFF_CHAPTER, LOG_CHAPTER, useDetailKeys } from "./detail-keys";
import { DetailSheet, holdOf, type OpenSheet, type SheetMove, type SheetReading } from "./Sheets";
import { chaptersOf } from "./chapters";
import { landingsOf, stepTimelineOf, turnsOfAttempt, wroteIn } from "./timeline";
import { PlanBar } from "./grouped";
import { keepingProduced } from "./produced-panel";
import type { AttemptRead } from "./timeline";
import type { StepChapter } from "@armada/components";
import { againOf, useShowAgain } from "./again";
import { leadOf } from "./lead";
import { OverviewBoard } from "./OverviewBoard";
import type { DetailTab } from "./detail-tabs";
import type { CheckAt } from "./tab-record";
import { CircleDashed } from "lucide-react";

import { shapeSaid } from "./plan-board";

import { workflowRunOf } from "./workflow-canvas";
import type { Figure } from "@armada/components";
import type { JobExamined } from "@armada/protocol";
import type { PulseView } from "./draft/pulse";
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
import { checkEntryId, useRunSheet } from "./rehearsal";
// Which Check's output `o` opens. **The same call the Checks chapter's own act
// makes**, so the key and the control cannot open different files.
import { checksOf, outputOf } from "./gates";
import { openKept } from "./phases";
import { CHECKS_CHAPTER } from "./checks";
import { runOf, whyNoSteps } from "./run";
import { answeringOf, commandOf, questionOf, waitingOf } from "./step";
import { taskGroupsOf } from "./draft/group";
import { entriesOf, hideUnread, whyNotWatching } from "./story";
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
import { whyNoBrief } from "./work";



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
  /**
   * Draw the proposal this Job is waiting to have approved. **Absent where
   * there is no proposal read to draw**, and then the lead offers no act —
   * a dead press is what `#1675` was filed against.
   */
  onOpenProposal?: () => void;
  /**
   * What a plan at the review gate takes that only the screen holds: a Drone
   * opened from a task's peek, the task to land on, and the way back —
   * `PlanTab`'s own three. Absent, the peek draws no Open.
   */
  onOpenDrone?: (droneId: string) => void;
  opensTask?: string;
  trail?: TrailProps;
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
    onReadCall,
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
    onShowAgain,
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
  // header can open one: `Sheets.tsx`'s `SheetReading`, and one value because
  // the log's attempt and its held position only ever change with the sheet.
  const onSheet = props.onSheet;
  const move = props.onMove;
  // Whether the folded inspector is up. Read only while `narrow`: above the
  // bound the inspector is a column and this decides nothing.
  const [, setInspecting] = useState(false);
  // Which members somebody dropped in this window. Nothing serves a drop, so
  // what a press changes is what is drawn — `useDroppedMembers` says why.
  const dropping = useDroppedMembers();
  const sheet = onSheet.which;
  const logAttempt = onSheet.which === "log" ? (onSheet.attempt ?? null) : null;
  const held = onSheet.which === "log" ? onSheet.held : null;
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
  // is still the expensive read, so the Produced chapter and the review
  // decision draw from this one answer.
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
  // What the observe socket says about itself, where it is not reading. **A
  // third answer the story needs**: four of the five states carry no rows, and
  // a chapter drawn from the rows alone reads every one of them as a step that
  // has not started. `story.ts` holds the sentences. #324.
  const transcript = whyNotWatching(observed);

  // What the keyboard can name, built before it is drawn. **The three regions
  // the contextual tier reaches are values here rather than queries later** —
  // the run, the story and the strip — which is what lets `detail-keys` open a
  // step, a chapter or a stage by name. #271.
  // The moment this Job's Drone handed in, where one arrived and it is this
  // Job's. A moment held for another Job would draw a submission under a step
  // that has not made one — `mine.ts`'s rule for every other read here. `#813`.
  const handed =
    recorded.handed.state === "heard" && recorded.handed.jobId === job.id
      ? recorded.handed.moment
      : undefined;
  const run =
    whole === null ? [] : runOf(whole, now, selected ?? undefined, watching?.rows ?? [], handed);
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
  // The open step's own submission, which the strip's Submitted tier draws and
  // its Judge tier points at. Read off the same `evidence` the trail below is
  // built from rather than fetched again — one call, drawn twice, which is the
  // rule `chapters.tsx` already follows for the same records.
  // `read` is the only state carrying rows, and a read that has not arrived is
  // not a step that claimed nothing — the tier draws its documents either way.
  // The criterion a live judge question holds open on this step. Read here and
  // handed to the story; the strip that also took it is gone.
  const asking =
    whole?.judge_question?.step_id === open?.step_id
      ? whole?.judge_question?.criterion_id
      : undefined;

  // The detail's contextual tier, and the open state it moves. Bound while a
  // Job is open and not before, so nothing on the Board listens for a key that
  // means nothing there — and the press is swallowed only where something
  // answered it. The story is read back through a function because it is built
  // from what this holds; see `DetailShape.chapters`.
  const keys = useDetailKeys({
    run,
    landings: () => landingsOf(timeline ?? []),
    // `f`, from `actions.toml` — `open_diff`, scope `detail`. It opens the
    // layer now rather than a chapter: the patch stopped being something the
    // panel draws. `Enter` needs nothing here, because `[` `]` land focus on
    // the chapter's own control and Enter is what a focused control already
    // answers — which is the reading `open_log`'s registry row gives it.
    onOpenSheet: () => openSheet("diff"),
    // `L`, from `actions.toml` — `open_log`, scope `detail`. **This is the line
    // that was missing.** The registry carried the key, `detail-keys` carried
    // the binding and dispatched it, and nothing here handed it a handler, so
    // the press found `undefined`, answered nothing and read as a key that was
    // never bound. `DetailShape` requires both openers now, so the next one
    // cannot go missing quietly.
    onOpenLog: () => openSheet("log"),
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

  // The rest of any call argument the socket cut, for as long as this Job is
  // open. **Held for the Job rather than for a log**, because the story draws
  // the same row twice — chapter one's turns and chapter two's preview — and a
  // fetch made in one is the same argument in the other.
  const calls = useCallArguments(onReadCall, job.id);

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
  // Asking the Job to show its work again, and the frames its presses kept on
  // the open step. `again.tsx` holds all of it.
  const pressing = useShowAgain(onShowAgain, job.id, whole?.show_again, open?.step_id, frames);

  const turns = watching === null ? [] : watching.rows;
  // The sheet's turns: one attempt's where it was opened from one, the step's
  // whole record otherwise.
  const read = open === undefined || logAttempt === null ? turns : turnsOfAttempt(open, logAttempt, turns);
  const rows = hideUnread(open === undefined ? [] : entriesOf(read, open.step_id)).rows;

  /**
   * Open a sheet. **The second one replaces the first** rather than stacking on
   * it. Opening the log on a live step starts following the tail — #1155 — and
   * the sheet itself is what holds once a person scrolls away from it, through
   * `onFollowingChange` in `Sheets.tsx`.
   */
  function openSheet(which: Exclude<OpenSheet, null | "check" | "task">, attempt?: number): void {
    // Held on open only where there is a tail and nothing is watching it: a run
    // that has ended does not grow, so the strip would offer a jump to nothing,
    // and a live run starts following instead of holding from the first paint.
    const holding =
      which === "log" && attempt === undefined && observed.state !== "watching"
        ? holdOf(now, rows.length)
        : undefined;
    move({
      move: "open",
      which,
      ...(attempt === undefined ? {} : { attempt }),
      ...(holding === undefined ? {} : { held: holding }),
    });
  }

  /** Open one plan task's reading. The rail draws a title and a count; `#1421`'s
   *  fields are this sheet's. */
  /**
   * Open the Check output sheet on the current attempt's Check — live where
   * the gate is still running it, kept once it has ruled. `checkSheetOf` in
   * `Sheets.tsx` is what reads that off the step; this only names the Check.
   * #1021.
   */
  function openCheck(checkId: string): void {
    move({ move: "open", which: "check", checkId });
  }

  /**
   * Close it, and put focus back where it came from. **The chapter line is the
   * way back** — `4k`'s third still — so `[` `]` carry on from the chapter the
   * reader opened rather than from the top of the story.
   *
   * **The holdings sheet lands nowhere, because it came from nowhere in the
   * story.** It opens from the run column rather than from a chapter, and
   * putting a reader who closed it onto a chapter they never opened would move
   * them further than `Esc` promised. Its control is the natural landing and
   * the summary is not part of the keyboard's chapter line. Reported.
   */
  function closeSheet(): void {
    const was = sheet;
    move({ move: "close" });
    if (was === "log" || was === "diff") {
      keys.onFocusChapter(was === "log" ? LOG_CHAPTER : DIFF_CHAPTER);
    }
    if (was === "check") keys.onFocusChapter(CHECKS_CHAPTER);
  }

  /**
   * The step's story, **built for one of its runs rather than for the step**.
   * `read` is that run narrowed by `asAttempt`, and every chapter narrows
   * itself to the attempt it is handed — so the timeline can ask for each.
   *
   * A run that is over gets nothing that means *right now*: no live mark, no
   * Judge's open question, no harness to run again, and no patch.
   */
  function storyOf(read: AttemptRead, over: boolean): StepChapter[] {
    if (open === undefined) return [];
    const step = read.step;
    const attempt = step.attempts[0]?.attempt;
    const ended = over ? wroteIn(read.turns) : undefined;
    return chaptersOf({
      job,
      whole,
      step,
      // Every step, in the frozen workflow's order, for the Drone brief's
      // `steps` section. Same nullable read as `criteria` below: a Job
      // whose detail has not arrived yet has no order to report.
      steps: whole?.steps ?? [],
      // The Job's frozen criteria, for the Verdicts chapter. The same list
      // the phase strip's Judge tier joins against, from the same reading.
      criteria: whole?.acceptance_criteria ?? [],
      watching: watching === null ? watching : { ...watching, rows: read.turns },
      footprint: recorded.footprint,
      kept: whole?.footprint,
      diff: recorded.diff,
      live: ended === undefined && observed.state === "watching",
      transcript,
      log: keys.inLog,
      calls,
      frames,
      ...(ended !== undefined
        ? {}
        : {
            again: againOf(
              onShowAgain === undefined ? undefined : whole?.show_again,
              open.step_id,
              frames,
              pressing,
            ),
          }),
      sheet: ended === undefined ? sheet : null,
      // The Produced chapter opens the step's deliverable, which the phase
      // strip's Submitted tier was the only route to. Same handler, because
      // two would be two vocabularies for one failed open — #307.
      opens: opensRecords,
      onOpenSheet: openSheet,
      onRedirect,
      now,
      // Scoped to the step `stuck` is actually about — a reader may have
      // navigated to a different step, and `stuck.undecided` is not that
      // step's reason for anything.
      undecided:
        ended === undefined && whole?.stuck?.step_id === open.step_id
          ? whole?.stuck?.undecided
          : undefined,
      asking: ended === undefined ? asking : undefined,
      onRunHere: (checkId) => runHook.open(checkEntryId(checkId)), // Journey 9
      // **The sheet only ever reads the current attempt.** `checkSheetOf`
      // reads `open` — this Job's current step — so a press on an earlier
      // attempt's row goes straight to the editor instead, the way it always
      // did: that attempt's own kept path, read off `step` here rather than
      // `open`, is still exactly attributable without the sheet's help.
      openCheckId: ended === undefined ? openCheckId : undefined,
      onOpenCheck:
        ended === undefined
          ? openCheck
          : (checkId) => {
              const kept = checksOf(step).find((one) => one.name === checkId)?.run?.output_path;
              if (kept !== undefined) openKept(opensRecords, { kept, what: "check" });
            },
      ...(attempt === undefined ? {} : { attempt }),
      ...(ended === undefined ? {} : { ended }),
      jobTurns: turns,
    });
  }

  // The timeline arranges what the story builds, run by run; it derives
  // nothing either of them holds.
  const bar = whole?.work_plan === undefined ? undefined : <PlanBar plan={whole.work_plan} />;
  const produced = keepingProduced(storyOf);
  const timeline = open && stepTimelineOf(open, turns, now, produced.storyOf, bar);

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
      onApproveReview: props.onApproveReview,
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
      now,
      onOpenCheck: (name, stepAttempt) => props.onOpenCheck({ name, stepAttempt }),
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
            rows={rows}
            {...(logAttempt === null ? {} : { ofAttempt: logAttempt })}
            // The turns those rows were folded from, which carry the tool and
            // the timing a row no longer does. The sheet folds runs of one tool
            // the way the chapter does, and this is what it folds them by.
            turns={read}
            observed={observed}
            diff={recorded.diff}
            calls={calls}
            // Its own name, so a row opened in the sheet is not a row opened in
            // the chapter's preview. Two logs over one stream hold equal ids.
            log={keys.inLog("sheet")}
            held={held}
            now={now}
            checkId={openCheckId}
            {...(onSheet.which === "task" ? { taskId: onSheet.taskId } : {})}
            {...(taskTouched === undefined ? {} : { taskTouched })}
            planTasks={plan?.recorded === true ? plan.tasks : undefined}
            outputs={outputs}
            following={following}
            onHold={(to) => move({ move: "hold", held: to })}
            onRedirect={onRedirect}
            // The Pulse board, derived exactly as the Pulse tab derives it —
            // `Refresh` came with it, because it acts on this reading and not
            // on the five lines that open it. Two derivations of one `Held`
            // would let the sheet and the tab disagree about the same Job.
            holds={{
              jobId: job.handle,
              reading: holding === null ? null : pulseReadingOf(pulseViewOf(holding), examinedNow),
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
  const waiting = waitingOf(
    questionOf(whole, job.id, stale, acting, onAnswer, actingAct),
    commandOf(whole, answering, explain),
    verdictSlot,
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
  // **And no act where the route cannot land.** The approval act reaches
  // `ProposalTab`, which draws nothing without a proposal read, so the screen
  // hands `onOpenProposal` down only where it has one.
  const opens = lead.opens;
  const openProposal = props.onOpenProposal;
  const reaches =
    opens === undefined || !("proposal" in opens) || openProposal !== undefined;
  const leadAct =
    lead.act === undefined || open === undefined || waiting !== undefined || !reaches ? undefined : (
      <Button
        onClick={() => {
          if (opens === undefined) {
            // Selecting the step is what opens the inspector on it, which is
            // where the question box and the gate's own acts already are.
            selectStep(open.step_id);
            return;
          }
          if ("check" in opens) props.onOpenCheck(opens.check);
          else if ("proposal" in opens) openProposal?.();
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
      lead={{ ...lead, act: leadAct }}
      waiting={waiting}
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
        ? { planAbsent: plan?.recorded === false ? `${plan.stepLabel} has not recorded one yet.` : undefined }
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
                said: GROUP_STATE[group.state]?.verb ?? group.state,
                status: GROUP_STATE[group.state]?.badgeStatus ?? "not-started",
                icon: GROUP_STATE[group.state]?.icon ?? CircleDashed,
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
      pulse={pulseCard(holding === null ? null : pulseViewOf(holding), examinedNow, whole)}
      pulseAbsent={whyNoReading(resources)}
      {...(whole === null ? {} : { brief: whole.facts })}
      briefAbsent={whyNoBrief(watched, job.id)}
      // What froze, and whether anybody has moved a setting since. `changedOf`
      // is the same count the strip's own Settings tab carries.
      settings={settingsSaid(props.draft?.landing, changedOf(whole))}
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