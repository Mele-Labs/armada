// Record — one ledger of everything that happened to this Job, newest first.
//
// **It replaced a column of regions, which replaced a strip.** The regions were
// the run tree and the brief folded under eyebrows; before that `JobRecord`
// held five readings behind a strip of its own. Reading why a step was refused
// meant opening four views and lining the timestamps up by hand, which is the
// whole of `#1537`.
//
// The rows are composed from today's reads in `draft/ledger.ts`, the words are
// `record.ts`, and this file holds the open state of one reading.

import { useEffect, useMemo, useState } from "react";
import { ChevronRight } from "lucide-react";
import {
  Button,
  CHECK_ADVANCES,
  ConsoleOutput,
  DropdownMenu,
  JobLedger,
  UnifiedDiff,
  WorkflowStepCard,
  type JobLedgerRow,
} from "@armada/components";
import type { Diff, JobDetail as JobWhole } from "@armada/protocol";

import { TAB_LABEL } from "./detail-tabs";
import { absoluteOf, clock } from "./duration";
import {
  filtersOf,
  FAMILY_SAYS,
  ledgerRowsFor,
  underFilter,
  underStep,
  unfiledSays,
  type RecordFilter,
} from "./record";
import { noteFor, regionOf, rowsOf, useCheckOutputs, type ReadCheckOutput } from "./outputs";
import { Eyebrow, FieldLabel } from "./regions";
import { titleOf } from "./record-cells";
import { taskGroupsOf, type GroupView } from "./draft/group";
import { familyOf, pathsOf, type LedgerRow } from "./draft/ledger";
import { drawn as drawnPatch, whyNoDiff } from "./review";
import { droneOfTask } from "./tab-plan-read";
import { stepNodeId, workflowRunOf } from "./workflow-canvas";
import { spentOf } from "./workflow-inspector";

export type RecordTabProps = {
  jobId: string;
  /** The Job whole, where Fleet answered for it. */
  detail: JobWhole | null;
  /** Every row of the Record, newest first. */
  rows: readonly LedgerRow[];
  /** The window is at `--window-floor`. */
  floor: boolean;
  onReadCheckOutput: ReadCheckOutput;
  /** The Job's diff, as the screen holds it. A File row draws its own files' part. */
  diff: Diff;
  /** Ask for the Job's diff, or `null` to let it go. Asked when a File row opens. */
  onReadDiff: (jobId: string | null) => void;
  /** The plan's groups, where the draft holds them. Absent reads the wire's. */
  groups?: readonly GroupView[];
  /** Say a sentence to the person — what an act that is not built yet answers. */
  onSaid: (sentence: string) => void;
  /**
   * Open a step in the Workflow destination, its panel open. **The screen's,
   * not this tab's** — `JobDetail.tsx` owns which destination is open.
   */
  onOpenStep: (stepId: string) => void;
};

export function RecordTab({
  jobId,
  detail,
  rows,
  floor,
  onReadCheckOutput,
  diff,
  onReadDiff,
  groups: given,
  onSaid,
  onOpenStep,
}: RecordTabProps) {
  const [filter, setFilter] = useState<RecordFilter>("all");
  // Which step the rows are narrowed to. `null` is every step, and the Job's
  // own rows with them.
  const [step, setStep] = useState<string | null>(null);
  // Which row is open. **Held here and not in the ledger**, so a live redraw of
  // the Record does not close the row somebody is reading.
  const [openRow, setOpenRow] = useState<string | null>(null);

  const outputs = useCheckOutputs(onReadCheckOutput, jobId);
  // The plan's groups, and every task in them, for the Drone a row names and
  // the step node a failed Check draws. The Workflow tab's own fallback.
  const groups = useMemo(() => given ?? (detail === null ? [] : taskGroupsOf(detail)), [given, detail]);
  const tasks = useMemo(() => groups.flatMap((group) => group.tasks), [groups]);

  const atStep = useMemo(() => underStep(rows, step), [rows, step]);
  const shown = useMemo(() => underFilter(atStep, filter), [atStep, filter]);
  const drawn = useMemo(() => ledgerRowsFor(shown, detail), [shown, detail]);
  // Nothing is open until a row is pressed: the reading is a sheet over the
  // table, and one nobody asked for would cover the rows it reads.
  const open = shown.find((row) => String(row.cursor) === openRow);
  const openDrawn = drawn.find((row) => row.id === openRow);
  // The steps a row names, in the Job's own order. A step nothing happened in
  // is not a place to narrow to.
  const steps = (detail?.steps ?? []).filter((one) =>
    rows.some((row) => row.coord?.step === one.step_id),
  );

  return (
    <div className="armada-detail-tab" role="tabpanel" aria-label={TAB_LABEL.record}>
      {detail === null ? (
        <p className="armada-inside__absent" role="note">
          Fleet has not answered for this job, so there is nothing to read back.
        </p>
      ) : (
        <JobLedger
          rows={drawn}
          filters={filtersOf(atStep)}
          filter={filter}
          {...(unfiledSays(atStep) === undefined ? {} : { note: unfiledSays(atStep) })}
          onFilter={(id) => {
            setFilter(id as RecordFilter);
            // The open row may not answer the new filter, and an inspector
            // showing a row the table no longer holds is the stale panel this
            // screen exists to end.
            setOpenRow(null);
          }}
          controls={
            steps.length < 2 ? undefined : (
              // A menu like the filter's beside it, so the head is one row of
              // one kind of control.
              <DropdownMenu
                align="start"
                triggerLabel={steps.find((one) => one.step_id === step)?.label ?? ANY_STEP}
                entries={[
                  { kind: "item", id: "", label: ANY_STEP, selected: step === null },
                  ...steps.map((one) => ({
                    kind: "item" as const,
                    id: one.step_id,
                    label: one.label,
                    selected: one.step_id === step,
                  })),
                ]}
                onSelect={(id) => {
                  setStep(id === "" ? null : id);
                  setOpenRow(null);
                }}
              />
            )
          }
          openRow={openRow}
          onOpenRow={setOpenRow}
          kindMarks={filter === "all"}
          floor={floor}
          emptyNote={EMPTY[filter]}
          {...(open === undefined ? {} : { inspectorTitle: titleOf(open) })}
          {...(open === undefined || openDrawn === undefined
            ? {}
            : {
                inspector: (
                  <RowRead
                    key={openDrawn.id}
                    row={open}
                    rows={rows}
                    drawn={openDrawn}
                    detail={detail}
                    outputs={outputs}
                    jobId={jobId}
                    diff={diff}
                    onReadDiff={onReadDiff}
                    tasks={tasks}
                    groups={groups}
                    onSaid={onSaid}
                    onOpenStep={onOpenStep}
                  />
                ),
              })}
        />
      )}
    </div>
  );
}

const ANY_STEP = "Any step";

/** What each filter says when it holds nothing. Never one sentence for nine. */
const EMPTY: Record<RecordFilter, string> = {
  all: "Nothing has happened on this Job yet.",
  job: "Nothing about the Job itself has been recorded yet.",
  evidence: "No Drone has submitted evidence on this Job.",
  files: "Nothing has written a file on this Job yet.",
  checks: "No Check has run on this Job yet.",
  judges: "No Judge has answered on this Job yet.",
  drones: "No Drone has opened a step on this Job yet.",
  tasks: "No task on this Job has moved yet.",
  tests: "No case has been run on this Job yet.",
};

type RowReadProps = {
  row: LedgerRow;
  /** Every row of the Record, so a failed Check can say whether a later run passed. */
  rows: readonly LedgerRow[];
  drawn: JobLedgerRow;
  detail: JobWhole;
  outputs: ReturnType<typeof useCheckOutputs>;
  jobId: string;
  diff: Diff;
  onReadDiff: (jobId: string | null) => void;
  tasks: readonly GroupView["tasks"][number][];
  groups: readonly GroupView[];
  onSaid: (sentence: string) => void;
  onOpenStep: (stepId: string) => void;
};

/**
 * One row, read whole.
 *
 * **A Check shows its output and what it stopped**, which is the reading the
 * four views could not put side by side: the run, the lines it printed, and
 * the step the run decided.
 */
function RowRead({
  row,
  rows,
  drawn,
  detail,
  outputs,
  jobId,
  diff,
  onReadDiff,
  tasks,
  groups,
  onSaid,
  onOpenStep,
}: RowReadProps) {
  const step = detail.steps.find((one) => one.step_id === row.coord?.step);
  const run =
    row.kind !== "checked"
      ? undefined
      : step?.check_runs.find(
          (one) => one.name === row.what && one.attempt === row.coord?.step_attempt,
        );
  const kept = run?.output_path;
  const name = kept === undefined ? undefined : basename(kept);
  const held = name === undefined ? undefined : outputs.of(name);
  const family = familyOf(row.kind);

  // Opening the row is the ask: the reading is what the pane is for, and a
  // second press to see it was the step the board does not draw.
  useEffect(() => {
    if (name !== undefined && outputs.of(name) === undefined) outputs.fetch(name);
  }, [name, outputs]);

  // A File row asks for the Job's diff when it opens, and never the Record:
  // the patch is the expensive read, and Overview lets it go on the way out.
  useEffect(() => {
    if (family !== "files") return;
    onReadDiff(jobId);
    return () => onReadDiff(null);
  }, [family, jobId]);

  const passedLater = run?.outcome === "failed" ? laterPass(row, rows) : undefined;
  // The step's node, as the Workflow canvas draws it — `workflowRunOf`'s own
  // card, so the two cannot say different things about one step.
  const node =
    step === undefined || run === undefined || CHECK_ADVANCES[run.outcome] !== false
      ? undefined
      : workflowRunOf({ whole: detail, groups, onOpen: () => onOpenStep(step.step_id) }).nodes.find(
          (one) => one.id === stepNodeId(step.step_id),
        )?.card;
  // Who ran it, where the eyebrow has not already said so: a Check's row is
  // run by a Check, and `Check · Implement` over `06:46:00 · Check` said it
  // twice. A Drone writing a File is a fact the eyebrow does not carry.
  const who = family !== null && drawn.whoSays === FAMILY_SAYS[family] ? null : drawn.whoSays;
  // The step, as a way to it: the eyebrow names it, and pressing it opens the
  // step's panel in Workflow. What follows the step — its group, its task —
  // stays words, after the step's chevron, which reads as the trail's own
  // separator rather than a `›` and a `·` side by side.
  const where = typeof drawn.where === "string" ? drawn.where : undefined;
  const after =
    step === undefined || where === undefined || !where.startsWith(step.label)
      ? undefined
      : where.slice(step.label.length).replace(/^ · /, " ");

  return (
    <div className="armada-ledger__read">
      <header className="armada-ledger__read-head">
        <Eyebrow>
          {family == null ? null : `${FAMILY_SAYS[family]} · `}
          {step === undefined || after === undefined ? (
            drawn.where
          ) : (
            <>
              <button
                type="button"
                className="armada-screen__eyebrow-act"
                onClick={() => onOpenStep(step.step_id)}
              >
                {step.label}
                <ChevronRight size={12} strokeWidth={2} aria-hidden />
              </button>
              {after}
            </>
          )}
        </Eyebrow>
        {/* The sheet's title names the row; this is drawn only where the title
            had to shorten it — a row of paths, read whole here. Not on a File
            row: each diff section below heads with its whole path. The owner,
            29 Sep. */}
        {family === "files" || titleOf(row) === row.what ? null : (
          <p className="armada-ledger__read-name">{row.what}</p>
        )}
        <p className="armada-ledger__read-status">
          {run === undefined ? null : (
            <span className="armada-ledger__read-outcome" data-tone={drawn.tone}>
              {row.outcome}
            </span>
          )}
          <span className="armada-ledger__read-meta" title={absoluteOf(row.at) ?? undefined}>
            {clock(row.at)}
            {who === null ? null : <> · {who}</>}
          </span>
        </p>
      </header>

      <div className="armada-ledger__read-body">
        {run === undefined ? (
          <>
            {/* A File row's outcome stays in the table; its sheet is for the
                diff. The owner, 29 Sep. */}
            {family === "files" || row.outcome === "" ? null : (
              <p className="armada-ledger__read-said">{row.outcome}</p>
            )}
            <DroneRead row={row} detail={detail} tasks={tasks} />
            {family === "files" ? <FileDiff row={row} diff={diff} jobId={jobId} /> : null}
          </>
        ) : (
          <>
            {/* What it produced, and what it was held to: two things, so two
                treatments — the result as body text, the bar as a labelled
                field under it. Run together they read as one sentence. */}
            {run.produced === undefined ? null : (
              <p className="armada-ledger__read-produced">{sentenceCase(run.produced)}</p>
            )}
            {run.expected === undefined ? null : (
              <div className="armada-ledger__read-field">
                <FieldLabel>Expected</FieldLabel>
                <p className="armada-ledger__read-said">{sentenceCase(run.expected)}</p>
              </div>
            )}

            <section className="armada-ledger__read-section">
              <Eyebrow>Output</Eyebrow>
              {/* Fetched when the row opens and never with the Record — a test
                  runner's whole output is what the split keeps off the
                  published state. */}
              {name === undefined ? (
                <p className="armada-ledger__note">This Check kept no output.</p>
              ) : held === undefined || held.state === "fetching" ? (
                <p className="armada-ledger__note">Reading what it printed</p>
              ) : (
                <ConsoleOutput
                  rows={held.state === "got" ? rowsOf(held.output) : []}
                  {...(held.state === "got" ? { region: regionOf(held.output) } : {})}
                  emptyNote={noteFor(held)}
                />
              )}
            </section>

            {/* Only where the run held its step: one that passed stopped
                nothing, and the eyebrow already reaches the step. Under the
                sentence, the step's own node off the Workflow canvas — what it
                is doing now, and pressing it opens its panel there. */}
            {step === undefined || node === undefined ? null : (
              <section className="armada-ledger__read-section">
                <Eyebrow>What it stopped</Eyebrow>
                <div className="armada-ledger__read-well">
                  <p className="armada-ledger__read-said">Blocked {step.label} from completing.</p>
                  <WorkflowStepCard {...node} />
                  {passedLater === undefined ? null : (
                    <p className="armada-ledger__read-later">
                      <span className="armada-ledger__read-dot" aria-hidden />
                      Passed at {clock(passedLater.at)}, on attempt {passedLater.coord?.step_attempt}
                    </p>
                  )}
                </div>
              </section>
            )}
          </>
        )}

        {/* Only where a gate on a group failed: running again is the answer
            to a failure, and a group nothing failed in has nothing to retry. */}
        {(family === "checks" || family === "judges") && drawn.tone === "failed" ? (
          <RunAgain row={row} detail={detail} onSaid={onSaid} />
        ) : null}
      </div>
    </div>
  );
}

/**
 * Which Drone produced a row, and what it spent. **The Workflow panel's own
 * reading** — `droneOfTask` for the label and `spentOf` for the figures — so
 * the two surfaces cannot name one Drone two ways. Nothing where the row names
 * no task, or the plan holds none by that id.
 */
function DroneRead({
  row,
  detail,
  tasks,
}: {
  row: LedgerRow;
  detail: JobWhole;
  tasks: RowReadProps["tasks"];
}) {
  const taskId = row.coord?.task;
  if (row.actor !== "drone" || taskId === undefined) return null;
  const task = tasks.find((one) => one.id === taskId);
  const drone = task === undefined ? undefined : droneOfTask(detail, task);
  if (task === undefined || drone === undefined) return null;
  const spent = spentOf(task);
  return (
    <section className="armada-ledger__read-section">
      <Eyebrow>Drone</Eyebrow>
      <p className="armada-ledger__read-drone">
        {drone.label} · {task.state}
      </p>
      {spent.length === 0 ? null : <p className="armada-ledger__read-said">{spent.join(" · ")}</p>}
    </section>
  );
}

/**
 * The part of the Job's diff a File row names.
 *
 * **The branch's change to the file, not the task's.** Fleet serves one patch
 * for the Job, so a file two tasks wrote shows both, and the eyebrow says whose
 * diff it is. Nothing while it is being read or where it holds no section for
 * the file; a read that failed keeps `whyNoDiff`'s words.
 */
function FileDiff({ row, diff, jobId }: { row: LedgerRow; diff: Diff; jobId: string }) {
  const mine = diff.state !== "none" && diff.jobId === jobId ? diff : null;
  const work = mine?.state === "read" ? mine.work : undefined;
  const patch = useMemo(() => (work === undefined ? undefined : drawnPatch(work)), [work]);
  if (mine?.state === "failed") {
    return (
      <section className="armada-ledger__read-section">
        <Eyebrow>{JOB_DIFF}</Eyebrow>
        <p className="armada-ledger__note">{whyNoDiff(diff, jobId)}</p>
      </section>
    );
  }
  if (patch === undefined) return null;
  const paths = pathsOf(row);
  const files = patch.files.filter((file) =>
    paths.some((path) => file.path === path || file.path.startsWith(path.endsWith("/") ? path : `${path}/`)),
  );
  if (files.length === 0) return null;
  // The bound can fall inside the last file drawn, so that file says so.
  const last = patch.files[patch.files.length - 1];
  const cut = patch.cut !== undefined && last !== undefined && files.includes(last) ? patch.cut : undefined;
  return (
    <section className="armada-ledger__read-section">
      <Eyebrow>{JOB_DIFF}</Eyebrow>
      <UnifiedDiff files={files} emptyNote="" {...(cut === undefined ? {} : { cut })} />
    </section>
  );
}

/** Whose diff a File row draws: the branch's, which any task may have added to. */
const JOB_DIFF = "Diff on the branch";

/** The same Check passing on a later run of the same step, where one did. */
function laterPass(row: LedgerRow, rows: readonly LedgerRow[]): LedgerRow | undefined {
  const at = row.coord;
  if (at === null) return undefined;
  return rows.find(
    (one) =>
      one.kind === "checked" &&
      one.what === row.what &&
      one.coord?.step === at.step &&
      one.coord.step_attempt > at.step_attempt &&
      one.outcome.toLowerCase().startsWith("passed"),
  );
}

function sentenceCase(said: string): string {
  return said.charAt(0).toUpperCase() + said.slice(1);
}

/**
 * Run this group again.
 *
 * **Mocked, and it says so when pressed.** No Fleet operation runs a group —
 * groups are not on the wire at all — so the control exists to be felt on the
 * mock and answers with what it would do rather than pretending to do it.
 */
function RunAgain({
  row,
  detail,
  onSaid,
}: {
  row: LedgerRow;
  detail: JobWhole;
  onSaid: (sentence: string) => void;
}) {
  const id = row.coord?.group;
  if (id === undefined) return null;
  const ordinal = taskGroupsOf(detail).find((group) => group.id === id)?.ordinal;
  if (ordinal === undefined) return null;
  const says = `Run group ${ordinal} again`;
  return (
    <Button
      variant="secondary"
      onClick={() => onSaid(`${says} — not built yet. Fleet has no operation for it.`)}
    >
      {says}
    </Button>
  );
}

// `outputs.fetch` is keyed by the file's own name and never the whole path —
// `fleet` builds that name out of the run's key, so a fixture or a record
// keyed on the path would never be found.
function basename(path: string): string {
  return path.slice(path.lastIndexOf("/") + 1);
}
