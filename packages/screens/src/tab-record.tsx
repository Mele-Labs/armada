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
  GroupBoundary,
  JobLedger,
  PathChip,
  RowLink,
  TaskMark,
  UnifiedDiff,
  WorkflowStepCard,
  type JobLedgerRow,
} from "@armada/components";
import type { Diff, JobDetail as JobWhole } from "@armada/protocol";

import { TAB_LABEL } from "./detail-tabs";
import { absoluteOf, clock } from "./duration";
import {
  filtersOf,
  ledgerRowsFor,
  rowSays,
  statusOf,
  underFilter,
  underStep,
  unfiledSays,
  type RecordFilter,
} from "./record";
import { noteFor, regionOf, rowsOf, useCheckOutputs, type ReadCheckOutput } from "./outputs";
import { Eyebrow, FieldLabel } from "./regions";
import { titleOf } from "./record-cells";
import { caseViewsOf, type CaseView } from "./draft/cases";
import { taskGroupsOf, type GroupView } from "./draft/group";
import { familyOf, pathsOf, type LedgerRow } from "./draft/ledger";
import { boundaryOf } from "./plan-board";
import { drawn as drawnPatch, whyNoDiff } from "./review";
import { droneOfTask, failedChecksOf, jobDroneOf, markOf, touchedByOf } from "./tab-plan-read";
import { stepNodeId, stepThatWorksTheGroups, workflowRunOf } from "./workflow-canvas";
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
  /** The cases the plan owes, where the draft holds them. Absent reads the wire's. */
  cases?: readonly CaseView[];
  /** Say a sentence to the person — what an act that is not built yet answers. */
  onSaid: (sentence: string) => void;
  /**
   * Open a step in the Workflow destination, its panel open. **The screen's,
   * not this tab's** — `JobDetail.tsx` owns which destination is open.
   */
  onOpenStep: (stepId: string) => void;
  /**
   * The Check whose row opens with the tab — by its name and the step attempt
   * that ran it — where another destination sent a person here. Read once.
   */
  opensCheck?: CheckAt;
};

/** A Check's run, as a boundary names it: its name, and the step attempt that ran it. */
export type CheckAt = { name: string; stepAttempt: number };

/**
 * The Record row a Check's run is: `checked`, that name, on the step the groups
 * are worked at, at that attempt. **Nothing where the Record holds no such
 * row.**
 */
function checkRowOf(rows: readonly LedgerRow[], detail: JobWhole | null, at: CheckAt): string | undefined {
  const step = detail === null ? undefined : stepThatWorksTheGroups(detail);
  if (step === undefined) return undefined;
  const row = rows.find(
    (one) =>
      one.kind === "checked" &&
      one.what === at.name &&
      one.coord?.step === step &&
      one.coord?.step_attempt === at.stepAttempt,
  );
  return row === undefined ? undefined : String(row.cursor);
}

/** A task's latest row in the Record — `rows` is newest first. Nothing where it has none. */
function taskRowOf(rows: readonly LedgerRow[], taskId: string): string | undefined {
  const row = rows.find((one) => familyOf(one.kind) === "tasks" && one.coord?.task === taskId);
  return row === undefined ? undefined : String(row.cursor);
}

export function RecordTab({
  jobId,
  detail,
  rows,
  floor,
  onReadCheckOutput,
  diff,
  onReadDiff,
  groups: given,
  cases: givenCases,
  onSaid,
  onOpenStep,
  opensCheck,
}: RecordTabProps) {
  const [filter, setFilter] = useState<RecordFilter>("all");
  // Which step the rows are narrowed to. `null` is every step, and the Job's
  // own rows with them.
  const [step, setStep] = useState<string | null>(null);
  // Which row is open. **Held here and not in the ledger**, so a live redraw of
  // the Record does not close the row somebody is reading.
  const [openRow, setOpenRow] = useState<string | null>(() =>
    opensCheck === undefined ? null : (checkRowOf(rows, detail, opensCheck) ?? null),
  );
  // A row pressed inside a reading — a Check on a task's boundary, a task a
  // Check held back — opens that row. **Every row, where the filter or the
  // step would hide it**: an open row the table does not hold is the stale
  // panel this screen exists to end.
  const openRowOf = (id: string | undefined) => {
    if (id === undefined) return;
    if (!underFilter(underStep(rows, step), filter).some((row) => String(row.cursor) === id)) {
      setFilter("all");
      setStep(null);
    }
    setOpenRow(id);
  };
  const openCheck = (name: string, stepAttempt: number) =>
    openRowOf(checkRowOf(rows, detail, { name, stepAttempt }));
  const openTask = (taskId: string) => openRowOf(taskRowOf(rows, taskId));

  const outputs = useCheckOutputs(onReadCheckOutput, jobId);
  // The plan's groups, and every task in them, for the Drone a row names and
  // the step node a failed Check draws. The Workflow tab's own fallback.
  const groups = useMemo(() => given ?? (detail === null ? [] : taskGroupsOf(detail)), [given, detail]);
  const tasks = useMemo(() => groups.flatMap((group) => group.tasks), [groups]);
  // What each case a task owes last came to — `casesOf`'s terms, the Plan tab's.
  const cases = useMemo(
    () => givenCases ?? (detail === null ? [] : caseViewsOf(detail)),
    [givenCases, detail],
  );

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
                    cases={cases}
                    onSaid={onSaid}
                    onOpenStep={onOpenStep}
                    onOpenCheck={openCheck}
                    onOpenTask={openTask}
                    taskRowOf={(taskId) => taskRowOf(rows, taskId)}
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
  cases: readonly CaseView[];
  onSaid: (sentence: string) => void;
  onOpenStep: (stepId: string) => void;
  onOpenCheck: (name: string, stepAttempt: number) => void;
  onOpenTask: (taskId: string) => void;
  /** A task's Record row, where it has one — which task lines are pressable. */
  taskRowOf: (taskId: string) => string | undefined;
};

/**
 * The group whose boundary a failed Check's run held back.
 *
 * **`failedChecksOf`'s own attribution, the one the boundary card draws**, and
 * no second rule: a run does not name its group on the wire, so it is the group
 * carrying a failure that runs this Check, read off the working step's latest
 * attempt. A run on any other step or attempt is not the one that attribution
 * reads, so it names no group.
 */
function groupHeldBy(
  detail: JobWhole,
  groups: readonly GroupView[],
  step: JobWhole["steps"][number],
  run: JobWhole["steps"][number]["check_runs"][number],
): GroupView | undefined {
  if (step.step_id !== detail.job.current_step_id) return undefined;
  const latest = Math.max(0, ...step.check_runs.map((one) => one.attempt));
  if (run.attempt !== latest) return undefined;
  return groups.find(
    (group) => group.checks_selected.includes(run.name) && failedChecksOf(detail, group).includes(run.name),
  );
}

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
  cases,
  onSaid,
  onOpenStep,
  onOpenCheck,
  onOpenTask,
  taskRowOf: taskRow,
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
  const status = statusOf(row);
  // The task a task row is about, read off the plan rather than the row: the
  // row carries the short reason, and the task carries the rest.
  const task =
    family === "tasks" && row.coord?.task !== undefined
      ? tasks.find((one) => one.id === row.coord?.task)
      : undefined;
  const diffPaths = family === "files" ? pathsOf(row) : (task?.scope ?? []);

  // Opening the row is the ask: the reading is what the pane is for, and a
  // second press to see it was the step the board does not draw.
  useEffect(() => {
    if (name !== undefined && outputs.of(name) === undefined) outputs.fetch(name);
  }, [name, outputs]);

  // A File row asks for the Job's diff when it opens, and never the Record:
  // the patch is the expensive read, and Overview lets it go on the way out.
  // A task row asks for it too, for its own files' part.
  const readsDiff = diffPaths.length > 0;
  useEffect(() => {
    if (!readsDiff) return;
    onReadDiff(jobId);
    return () => onReadDiff(null);
  }, [readsDiff, jobId]);

  const passedLater = run?.outcome === "failed" ? laterPass(row, rows) : undefined;
  // The group the run held back, where the boundary card names one: a Check
  // that stopped its step stopped that group's tasks with it (the owner, 29 Sep
  // 2026: *not just the step from completing but a task in the plan*).
  const heldGroup =
    step === undefined || run === undefined || CHECK_ADVANCES[run.outcome] !== false
      ? undefined
      : groupHeldBy(detail, groups, step, run);
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
  const says = rowSays(row);
  // A task's Drone by its own name, as the Plan and Workflow panels say it.
  const taskDrone = task === undefined || row.actor !== "drone" ? undefined : droneOfTask(detail, task);
  const whoSays = taskDrone?.label ?? drawn.whoSays;
  const who = whoSays === says ? null : whoSays;
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
          {says === undefined ? null : `${says} · `}
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
        {/* What happened leads, as a Check's `Failed` did before every row
            had a word. The owner, 29 Sep. */}
        <p className="armada-ledger__read-status">
          {status === undefined ? null : (
            <span className="armada-ledger__read-outcome" data-tone={status.tone}>
              {status.says}
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
                diff. Every other value is under its own label. The owner, 29 Sep. */}
            {task !== undefined ? (
              <TaskRead
                row={row}
                task={task}
                groups={groups}
                cases={cases}
                detail={detail}
                onOpenCheck={onOpenCheck}
              />
            ) : family === "files" ? null : (
              <RowFields row={row} detail={detail} />
            )}
            {task === undefined ? null : <FileDiff paths={diffPaths} diff={diff} jobId={jobId} />}
            <DroneRead row={row} detail={detail} tasks={tasks} />
            {family === "files" ? <FileDiff paths={diffPaths} diff={diff} jobId={jobId} /> : null}
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
                is doing now, and pressing it opens its panel there. Under that,
                the group the boundary card names as failed and its tasks, each
                opening its own row. */}
            {step === undefined || node === undefined ? null : (
              <section className="armada-ledger__read-section">
                <Eyebrow>What it stopped</Eyebrow>
                <div className="armada-ledger__read-well">
                  <p className="armada-ledger__read-said">Blocked {step.label} from completing.</p>
                  <WorkflowStepCard {...node} />
                  {heldGroup === undefined ? null : (
                    <>
                      <p className="armada-ledger__read-said">Blocked group {heldGroup.ordinal} from passing.</p>
                      <ul className="armada-ledger__read-said armada-ledger__read-list">
                        {heldGroup.tasks
                          .filter((task) => task.state !== "dropped")
                          .map((task) => {
                            const opens = taskRow(task.id);
                            return (
                              <li key={task.id}>
                                <RowLink
                                  mark={<TaskMark state={markOf(task.state)} />}
                                  {...(opens === undefined ? {} : { onOpen: () => onOpenTask(task.id) })}
                                >
                                  {`${task.id} · ${task.title}`}
                                </RowLink>
                              </li>
                            );
                          })}
                      </ul>
                    </>
                  )}
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
  if (familyOf(row.kind) === "drones") return <StepDroneRead row={row} detail={detail} />;
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
 * The Drone a Drone row is about. **The Job's one Drone**, because today's wire
 * holds one per step attempt and names none on the attempt itself — the same
 * fallback `droneOfTask` takes. Its state is the attempt's; no turns or cost,
 * which the wire keeps per task and never per attempt.
 */
function StepDroneRead({ row, detail }: { row: LedgerRow; detail: JobWhole }) {
  const drone = jobDroneOf(detail);
  if (drone === undefined) return null;
  const attempt = detail.steps
    .find((one) => one.step_id === row.coord?.step)
    ?.attempts.find((one) => one.attempt === row.coord?.step_attempt);
  return (
    <section className="armada-ledger__read-section">
      <Eyebrow>Drone</Eyebrow>
      <p className="armada-ledger__read-drone">
        {drone.label}
        {/* A Drone ending says what it came to under Outcome, above. */}
        {attempt === undefined || row.kind === "drone_exited" ? null : <> · {attempt.outcome}</>}
      </p>
    </section>
  );
}

/**
 * Every value a row carries past its title, each under its own label (the owner,
 * 29 Sep 2026: *there is just some text with no labels*). **One system for the
 * whole body**: a single value takes `FieldLabel` over it, as a Check's
 * `Expected` does, and a block — a list, a log, a diff, a Drone — takes the
 * `Eyebrow` over it. The two sit at one spacing, so the body is one column. A label the concepts table knows carries its
 * sentence on hover, which is where an explanation goes — never the body.
 *
 * Nothing at all where the row carries nothing more, so a Job created or a
 * Drone started is its status line and no placeholder.
 */
function RowFields({ row, detail }: { row: LedgerRow; detail: JobWhole }) {
  if (row.kind === "plan_recorded") {
    const planned = detail.work_plan?.tasks ?? [];
    if (planned.length === 0) return null;
    return (
      <section className="armada-ledger__read-section">
        <Eyebrow>Tasks</Eyebrow>
        <ol className="armada-ledger__read-said armada-ledger__read-list">
          {planned.map((task) => (
            <li key={task.id}>
              {task.id} — {task.title}
            </li>
          ))}
        </ol>
      </section>
    );
  }
  if (row.kind === "judged") {
    const [, found] = splitOnce(row.outcome, " — ");
    return (
      <>
        <Field label="Criterion" value={splitOnce(row.what, " · ")[1] ?? row.what} />
        {found === undefined ? null : <Field label="What it found" value={found} />}
      </>
    );
  }
  const label = FIELD_OF[row.kind] ?? "Outcome";
  return row.outcome === "" ? null : <Field label={label} value={row.outcome} />;
}

/** The label over a row's outcome, by kind. Anything unlisted reads `Outcome`. */
const FIELD_OF: Readonly<Record<string, string>> = {
  task_done: "What it showed",
  task_working: "What it showed",
  task_failed: "Why",
  task_dropped: "Why",
  flagged: "Cited",
  evidence_submitted: "Shown by",
  handed_in: "Evidence type",
};

function Field({ label, value }: { label: string; value: string }) {
  return (
    <div className="armada-ledger__read-field">
      <FieldLabel>{label}</FieldLabel>
      <p className="armada-ledger__read-said">{sentenceCase(value)}</p>
    </div>
  );
}

// The part before the first `separator` and the rest, or the whole and nothing.
function splitOnce(said: string, separator: string): [string, string | undefined] {
  const at = said.indexOf(separator);
  return at < 0 ? [said, undefined] : [said.slice(0, at), said.slice(at + separator.length)];
}

/**
 * A task row, read whole: **the action, why, and what supports it** (the
 * owner, 29 Sep 2026: *if there's any supporting evidence that should be
 * outlined*). The status line carries the action; this is the rest.
 *
 * **Why** is `expects` over `shown`, stacked because a Drone writes either at
 * any length, or the reason it failed or was dropped. **Evidence** is its
 * files, its cases, its group's Checks and a later task editing it.
 *
 * **The Checks are the group's, drawn by the Plan board's `GroupBoundary`** off
 * `boundaryOf`, so the Record and the Plan cannot disagree about a boundary
 * (the owner, 29 Sep 2026). Its tests strip is not drawn: the task's own cases
 * are `Tests` above. **Pressing a Check opens its own row.**
 */
function TaskRead({
  row,
  task,
  groups,
  cases,
  detail,
  onOpenCheck,
}: {
  row: LedgerRow;
  task: GroupView["tasks"][number];
  groups: readonly GroupView[];
  cases: readonly CaseView[];
  detail: JobWhole;
  onOpenCheck: (name: string, stepAttempt: number) => void;
}) {
  const owed = task.cases
    .map((id) => cases.find((one) => one.id === id))
    .filter((one): one is CaseView => one !== undefined);
  const group = groups.find((one) => one.id === task.group);
  const checks = group?.checks_selected ?? [];
  // The later task's words are the touched row's own reason; on any other
  // row of the task they are evidence.
  const touched =
    task.touched_after_done && row.kind !== "touched_after_done" ? laterSaid(task, groups) : undefined;
  const evidence = task.scope.length > 0 || owed.length > 0 || checks.length > 0 || touched !== undefined;
  return (
    <>
      {row.kind === "task_done" && (task.expects !== undefined || task.shown !== undefined) ? (
        <>
          {task.expects === undefined ? null : <Field label="Expected" value={task.expects} />}
          {task.shown === undefined ? null : <Field label="Shown" value={task.shown} />}
        </>
      ) : row.kind === "task_failed" && task.failed_reason !== undefined ? (
        <Field label="Why" value={task.failed_reason} />
      ) : row.kind === "task_dropped" && task.reason !== undefined ? (
        <Field label="Why" value={task.reason} />
      ) : row.kind === "touched_after_done" ? (
        <Field label="Why" value={row.outcome} />
      ) : null}

      {!evidence ? null : (
        <section className="armada-ledger__read-section">
          <Eyebrow>Evidence</Eyebrow>
          {task.scope.length === 0 ? null : (
            <div className="armada-ledger__read-field">
              <FieldLabel>Files</FieldLabel>
              <span className="armada-ledger__files">
                {task.scope.map((path) => (
                  <PathChip key={path} basename={basename(path)} title={path} />
                ))}
              </span>
            </div>
          )}
          {owed.length === 0 ? null : (
            <div className="armada-ledger__read-field">
              <FieldLabel>Tests</FieldLabel>
              <ul className="armada-ledger__read-said armada-ledger__read-list">
                {owed.map((one) => {
                  const result = caseResultOf(one);
                  return (
                    <li key={one.id} className="armada-ledger__read-line">
                      <PathChip basename={basename(one.spec)} title={one.spec} />
                      <span className="armada-ledger__read-outcome" data-tone={result.tone}>
                        {result.says}
                      </span>
                    </li>
                  );
                })}
              </ul>
            </div>
          )}
          {group === undefined || checks.length === 0 ? null : (
            <div className="armada-ledger__read-field">
              <FieldLabel>{`Group ${group.ordinal}'s boundary`}</FieldLabel>
              {/* No cases passed: the task's own are `Tests` above. */}
              <GroupBoundary
                {...boundaryOf(
                  group,
                  [],
                  detail,
                  detail.steps.find((one) => one.step_id === stepThatWorksTheGroups(detail)),
                  onOpenCheck,
                )}
              />
            </div>
          )}
          {touched === undefined ? null : <Field label="Changed after done" value={touched} />}
        </section>
      )}
    </>
  );
}

/** What a case last came to, in a word. A run has no verdict — see `CaseRunView`. */
function caseResultOf(one: CaseView): { says: string; tone?: "failed" } {
  const run = one.last_run;
  if (run === undefined) return { says: one.has_spec ? "Not run yet" : "Not covered" };
  if (run.outcome === "run_failed") return { says: "Run failed", tone: "failed" };
  if (run.outcome === "not_run") return { says: one.has_spec ? "Not run" : "Not covered" };
  return { says: run.frames > 0 ? `Ran, ${run.frames} frames` : "Ran" };
}

// The later task that edited a file this one had finished, in words — the
// Plan board's own join, `touchedByOf`.
function laterSaid(task: GroupView["tasks"][number], groups: readonly GroupView[]): string {
  const later = touchedByOf(groups).get(task.id);
  return later === undefined
    ? "A later task edited a file it had finished"
    : `${later} edited a file it had finished`;
}

/**
 * The part of the Job's diff a File row names.
 *
 * **The branch's change to the file, not the task's.** Fleet serves one patch
 * for the Job, so a file two tasks wrote shows both, and the eyebrow says whose
 * diff it is. Nothing while it is being read or where it holds no section for
 * the file; a read that failed keeps `whyNoDiff`'s words.
 */
function FileDiff({ paths, diff, jobId }: { paths: readonly string[]; diff: Diff; jobId: string }) {
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
  if (patch === undefined || paths.length === 0) return null;
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
