// The readings the panel cannot hold, on the layer that can — #286, and
// Journey 4's frames 4i-4m.
//
// The diff is the Job's whole patch, and a patch in a 602px column is a
// decision taken on a line that wrapped. The activity log was the other reading
// here until 2 Oct 2026, when the owner removed it: nothing on a Job is a single
// log, so a sheet that claimed to be one went with the `L` that opened it.
//
// **One sheet at a time, and `Esc` returns to the panel** rather than to the
// previous sheet. Which one is open is `JobDetail`'s state; what closes one is
// `Sheet` itself, which catches `Esc` in the capture phase so the other clause
// of the same registry row — *returns to the list from a detail route* — does
// not answer the same press.
//
// **Two exits and no third.** The labelled control and `Esc`. A click on the
// ground behind does not close a sheet.

import {
  ConsoleOutput,
  EvidenceSheet,
  JobDiffSheet,
  JobHoldsSheet,
  PlanTaskSheet,
  type PlanTaskSheetProps,
  RunSheet,
  railOfPatch,
  type JobDiffFile,
  type JobHoldsSheetProps,
  type RunSheetProps,
} from "@armada/components";
import { useEffect, useMemo } from "react";
import { usePulseWatch } from "./tab-pulse";

import type { Diff } from "@armada/protocol";
import type { JobDetail as JobWhole, JobSummary, StepDetail } from "@armada/protocol";
import type { PlanTaskRow } from "./plan";
import { checkSheetOf } from "./checks";
import { DecidedDiff } from "./Decide";
import {
  liveNoteFor,
  liveRegionOf,
  liveRowsOf,
  noteFor,
  regionOf,
  rowsOf,
  type Following,
  type Outputs,
} from "./outputs";
import { drawn, WORKTREE_GIVEN_BACK } from "./review";

/**
 * Which sheet is open, or none. Two cannot be.
 *
 * **`holds` is here for a different reason than the diff.** The diff left the
 * panel because a reading has no end; this left the run
 * column because it was the largest thing on it and the run is what a person
 * opens a Job to read. Same layer, same two exits, same one-at-a-time rule.
 *
 * **Settings was the fourth and is a destination now.** It came here because
 * the header's one line for one of them read as the screen's main button; the
 * owner made the whole panel the strip's sixth entry on 28 September 2026, so
 * there is no layer to leave and `tab-settings.tsx` holds it.
 *
 * **`run` is the fourth, Journey 9's.** It opens from `r`, from the worktree
 * row's `Run…`, and from a refused Check's `Run it here` — never from a
 * chapter, so it lands nowhere on close, `holds`'s way.
 *
 * **`check` is the sixth**: one Check's output, from its row, closing onto the Checks chapter.
 */
export type OpenSheet = "diff" | "holds" | "run" | "check" | "task" | null;

export type DetailSheetProps = {
  which: OpenSheet;
  job: JobSummary;
  whole: JobWhole | null;
  /** The step the panel is showing, restated here: the tree is under the layer. */
  step: StepDetail;
  diff: Diff;
  /**
   * Which Check the output sheet is open on — `which === "check"`'s own
   * reading. `checkSheetOf(step, checkId)` is what turns this into live or
   * kept; absent whenever `which` is not `"check"`.
   */
  checkId?: string;
  /**
   * What each Check on this Job has printed, where somebody opened one, and
   * how to ask for the rest. **Held for the Job**, because a recorded output
   * never moves — the sheet's own fetch, not the chapter's any more. #1021.
   */
  outputs: Outputs;
  /**
   * The running Check's log this window is following, and how to follow one.
   * **Started when the sheet mounts on a live Check and stopped when it
   * unmounts** — closing the sheet or switching Jobs both unmount it, which is
   * what lets go of the socket now that the chapter no longer does.
   */
  following: Following;
  /**
   * The full machine reading, exactly as the panel used to draw it — every
   * state and every argument, `Refresh` included. Built by the caller, because
   * the arguments for how to read a `Holds` live in `resources.ts`.
   */
  holds: Omit<JobHoldsSheetProps, "open" | "floor" | "onClose">;
  /**
   * Hold Pulse's reading live while the holds sheet is the one open, and let it
   * go otherwise. Same board and same poll as the Pulse tab, which is why it is
   * the same prop — `tab-pulse.tsx`, `#1571`.
   */
  onNeedPulse: (jobId: string | null) => void;
  /**
   * Which task the task sheet is reading, where one is open.
   *
   * **Named rather than handed the task.** `SheetReading` carries the id the
   * way it carries a Check's, so the sheet reads the plan as it stands now —
   * a task marked done while its sheet is open says done.
   */
  taskId?: string;
  /** Every task of the plan, for the sheet to find `taskId` among. */
  planTasks?: readonly PlanTaskRow[];
  /**
   * What the open task declared against what it touched, already read.
   *
   * **Built by the caller**, like `holds` and `run`: the inputs are the Job's
   * own turns and its diff, which `JobDetail` holds and this layer does not.
   * **Absent draws no comparison at all** rather than one saying every
   * declared file went untouched. `#1432`.
   */
  taskTouched?: PlanTaskSheetProps["touched"];
  /**
   * The run sheet, Journey 9 — built by the caller from `RunSheetRead`,
   * `RunFollowed` and the run-sheet's own selection state, `holds`'s reason:
   * the arguments belong to `rehearsal.ts` and `JobDetail`, not to this file.
   */
  run: Omit<RunSheetProps, "open" | "floor" | "onClose">;
  /** The window is at `--window-floor`. */
  floor: boolean;
  onClose: () => void;
};

export function DetailSheet({
  which,
  job,
  whole,
  step,
  diff,
  checkId,
  taskId,
  planTasks = [],
  taskTouched,
  outputs,
  following,
  holds,
  onNeedPulse,
  run,
  floor,
  onClose,
}: DetailSheetProps) {
  // Above the early returns, because a hook cannot be conditional.
  // `null` for every other sheet, which is what stops the reading being polled
  // for a board that is not drawn.
  usePulseWatch(which === "holds" ? job.id : null, onNeedPulse);
  if (which === "diff") {
    return <DiffSheet job={job} whole={whole} diff={diff} floor={floor} onClose={onClose} />;
  }
  if (which === "check" && checkId !== undefined) {
    return (
      <CheckSheet
        job={job}
        step={step}
        checkId={checkId}
        outputs={outputs}
        following={following}
        floor={floor}
        onClose={onClose}
      />
    );
  }
  if (which === "task" && taskId !== undefined) {
    const task = planTasks.find((one) => one.id === taskId);
    // A task the plan no longer holds draws nothing rather than an empty
    // sheet: a recording replaces the plan whole, so an id can go.
    return task === undefined ? null : (
      <PlanTaskSheet
        open
        id={task.id}
        title={task.title}
        state={task.state}
        reason={task.reason}
        scope={task.scope}
        {...(taskTouched === undefined ? {} : { touched: taskTouched })}
        expects={task.expects}
        shown={task.shown}
        floor={floor}
        onClose={onClose}
      />
    );
  }
  if (which === "holds") {
    return <JobHoldsSheet open floor={floor} onClose={onClose} {...holds} />;
  }
  if (which === "run") {
    return <RunSheet open floor={floor} onClose={onClose} {...run} />;
  }
  return null;
}

/**
 * The patch, the rail beside it and the count over both, from one reading.
 *
 * **Its own component so the split is not paid for by the other sheets.** The
 * parse is held across renders, and the panel above ticks `now` every second: a
 * 2,000-line patch re-split on every tick is the freeze the v1 failure log
 * recorded nine times. A hook in `DetailSheet` would have to run before its
 * early return and would run on every other sheet's render too.
 */
function DiffSheet({
  job,
  whole,
  diff,
  floor,
  onClose,
}: {
  job: JobSummary;
  /** Only for the footprint, which says whether there was a worktree to lose. */
  whole: JobWhole | null;
  diff: Diff;
  floor: boolean;
  onClose: () => void;
}) {
  const files = useMemo(() => railOf(diff, job.id), [diff, job.id]);
  return (
    <JobDiffSheet
      open
      floor={floor}
      branch={job.branch ?? job.handle}
      files={files}
      // **Which silence this is**, where the reading can say. #381.
      whyNoReading={gaveBackTheWorktree(diff, job.id, whole) ? WORKTREE_GIVEN_BACK : undefined}
      // What the counts are counted against. Absent on a peer built before
      // 7.9, where `measured_whole` defaults to true and the header falls back
      // to the neutral phrase rather than claiming a base nothing named.
      measuredFrom={diff.state === "read" ? diff.work?.measured_from : undefined}
      measuredWhole={diff.state === "read" ? diff.work?.measured_whole : undefined}
      note={WHICH_STEP_WROTE_IT}
      onClose={onClose}
    >
      <DecidedDiff diff={diff} jobId={job.id} />
    </JobDiffSheet>
  );
}

/**
 * One Check's output, on the layer that can hold it — #1021.
 *
 * **`EvidenceSheet`, wired in for the first time.** It existed as a component
 * and a story — `AChecksConsoleOutput` — and neither was built into the screen
 * that ships. A check's console output is the artifact that sheet was drawn
 * for.
 *
 * **`checkSheetOf` decides live or kept, every render.** A Check open in this
 * sheet while the gate rules moves from one to the other without the sheet
 * closing — the same Check, a different file, which is why this asks fresh
 * rather than fixing the answer at open.
 */
function CheckSheet({
  job,
  step,
  checkId,
  outputs,
  following,
  floor,
  onClose,
}: {
  job: JobSummary;
  step: StepDetail;
  checkId: string;
  outputs: Outputs;
  following: Following;
  floor: boolean;
  onClose: () => void;
}) {
  const read = checkSheetOf(step, checkId);
  return (
    <EvidenceSheet
      open
      floor={floor}
      kind="Console output"
      name={`${checkId} — output`}
      step={step.label}
      jobId={job.handle}
      onClose={onClose}
    >
      {read === undefined ? (
        <ConsoleOutput rows={[]} emptyNote={NOTHING_TO_READ} />
      ) : read.kind === "live" ? (
        <LiveCheckOutput kept={read.kept} following={following} />
      ) : (
        <KeptCheckOutput kept={read.kept} outputs={outputs} />
      )}
    </EvidenceSheet>
  );
}

/**
 * The kept file, read where the Check is. **The fetch is the open, and it
 * happens once** — `outputs.fetch` drops a second ask for a name it already
 * holds, so mounting this is what asks for the file rather than a press
 * inside it.
 */
function KeptCheckOutput({ kept, outputs }: { kept: string; outputs: Outputs }) {
  useEffect(() => outputs.fetch(kept), [outputs, kept]);
  const held = outputs.of(kept);
  const output = held?.state === "got" ? held.output : undefined;
  return (
    <ConsoleOutput
      rows={output === undefined ? [] : rowsOf(output)}
      {...(output === undefined ? {} : { region: regionOf(output) })}
      emptyNote={noteFor(held)}
    />
  );
}

/**
 * One running Check's log, followed as it is written. **Followed while the
 * sheet is on screen and let go the moment it is not** — mounting and
 * unmounting this is the whole of starting and stopping the socket, so
 * closing the sheet or switching Jobs both end it.
 */
function LiveCheckOutput({ kept, following }: { kept: string; following: Following }) {
  const { follow } = following;
  useEffect(() => {
    follow(kept);
    return () => follow(null);
  }, [follow, kept]);
  const region = liveRegionOf(following.reading, kept);
  return (
    <ConsoleOutput
      rows={liveRowsOf(following.reading, kept)}
      {...(region === undefined ? {} : { region })}
      emptyNote={liveNoteFor(following.reading, kept)}
    />
  );
}

/** A Check named for the sheet that no longer has anything behind it. */
const NOTHING_TO_READ = "This Check has nothing recorded to read.";

/**
 * Whether the missing reading is a worktree that was given back.
 *
 * **`work: None` is two facts and only one of them may be named.** Fleet
 * answers it whenever `worktree_of` finds nothing, which covers a Job whose
 * worktree was reclaimed after it finished *and* a Job that never got one —
 * one that failed before a Drone was placed. Saying `the worktree was given
 * back` over the second is the same false certainty #381 was filed about,
 * pointed the other way, so the phrase needs evidence rather than a default.
 *
 * The footprint is that evidence. Fleet writes it from the worktree at the
 * instant the Job stopped, so a Job holding one had a worktree to read and no
 * longer has it. Absent, the header keeps the neutral phrase — which is
 * honest, because absent is exactly where Bridge does not know.
 */
export function gaveBackTheWorktree(diff: Diff, jobId: string, whole: JobWhole | null): boolean {
  if (diff.state !== "read" || diff.jobId !== jobId || diff.work !== undefined) return false;
  return whole?.footprint !== undefined;
}

/**
 * Which sheet is up, and what it is reading — one value, so a sheet that closes
 * cannot leave its reading behind for the next sheet to inherit.
 */
export type SheetReading =
  | { which: null }
  | { which: "check"; checkId: string }
  | { which: "task"; taskId: string }
  | { which: Exclude<OpenSheet, "check" | "task" | null> };

/** What can happen to it: a sheet goes up or comes down. */
export type SheetMove =
  | { move: "open"; which: "check"; checkId: string }
  | { move: "open"; which: "task"; taskId: string }
  | { move: "open"; which: Exclude<OpenSheet, "check" | "task" | null> }
  | { move: "close" };

export const NO_SHEET: SheetReading = { which: null };

/** The next reading. A second sheet replaces the first. */
export function sheetMoved(_was: SheetReading, move: SheetMove): SheetReading {
  if (move.move === "close") return NO_SHEET;
  if (move.which === "check") return { which: "check", checkId: move.checkId };
  if (move.which === "task") return { which: "task", taskId: move.taskId };
  return { which: move.which };
}

/**
 * The file rail beside the patch — the paths, and what each gained and lost.
 *
 * **From the patch, which is the answer the body is drawn from.** It used to
 * come from the footprint, and a footprint is a step's read-back written when
 * the step submits: mid-step nothing has submitted, so the rail was empty and
 * the header read `0 files · +0 −0` above a fully rendered patch. That is
 * #310, and it was two sources on one line rather than a hole to plug — filling
 * the rail from the patch and leaving the counts on the footprint would have
 * kept the contradiction one field along.
 *
 * `drawn` is the same split `DecidedDiff` renders, called on the same reading,
 * so the rail names exactly the files beside it in the order the patch wrote
 * them. It is a second call of one pure function rather than a second answer.
 *
 * **`null` is no reading and `[]` is a reading of nothing**, and the split
 * falls on the line the wire already draws. `work` absent is a Job with no
 * worktree; `work` present with no patch is a drone that changed nothing, which
 * is a real answer and truthfully reads `0 files · +0 −0`. Returning `[]` for
 * both would put a count of nothing over a Job nothing was read from, which is
 * this issue one state over.
 *
 * **No step against a file.** The drawing names the step that wrote each one
 * and nothing served says which step that was: the footprint carries
 * `planned_by`, which is the step that *promised* a path, and a file no step
 * declared would then read as a file no step wrote. The rail draws the counts
 * alone and says why underneath rather than guessing. Reported.
 */
function railOf(diff: Diff, jobId: string): JobDiffFile[] | null {
  // A reading of some other Job is not this Job's reading. `whyNoDiff` is the
  // sentence the body carries for each of these, and the header says only that
  // it has none.
  if (diff.state !== "read" || diff.jobId !== jobId || diff.work === undefined) return null;
  return railOfPatch(drawn(diff.work).files);
}

/** What the rail says instead of naming a step, because nothing serves one. */
const WHICH_STEP_WROTE_IT =
  "Fleet commits once at the end, so the patch is the Job's. Nothing served says which step " +
  "wrote each file.";
