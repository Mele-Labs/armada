// Record — one ledger of everything that happened to this Job, newest first.
//
// **It replaced a column of regions, which replaced a strip.** The regions were
// the run tree and the brief folded under eyebrows; before that `JobRecord`
// held five readings behind a strip of its own. Reading why a step was refused
// meant opening four views and lining the timestamps up by hand, which is the
// whole of `#1537`.
//
// The rows are composed from today's reads in `draft/ledger.ts`, the words are
// `record.ts`, and this file holds the open state of one reading. The open
// row's reading is `record-read.tsx`, and the modules it names.

import { useEffect, useMemo, useState } from "react";
import { DropdownMenu, JobLedger } from "@armada/components";
import type { Diff, JobDetail as JobWhole } from "@armada/protocol";

import { TAB_LABEL } from "./detail-tabs";
import {
  filtersOf,
  ledgerRowsFor,
  underFilter,
  underStep,
  unfiledSays,
  type RecordFilter,
} from "./record";
import { useCheckOutputs, type ReadCheckOutput } from "./outputs";
import { titleOf } from "./record-cells";
import { caseViewsOf, type CaseView } from "./draft/cases";
import { taskGroupsOf, type GroupView } from "./draft/group";
import { familyOf, type LedgerRow } from "./draft/ledger";
import { stepThatWorksTheGroups } from "./workflow-canvas";
import { RowRead } from "./record-read";
import type { TrailProps } from "./trail";

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
  /** The row to open with the tab, by its id — where the way back returns to. Read once. */
  opensRow?: string;
  /**
   * The way back, where a press in another destination's panel landed here,
   * and where this one's open panel is reported — `trail.ts`.
   */
  trail?: TrailProps;
};

/**
 * A Check's run, as a boundary names it: its name, and the step attempt that
 * ran it.
 *
 * **The step, where the caller knows one.** A group boundary's Checks all run
 * on the step that works the groups, so the Plan names none; Overview's lead
 * names a Check that failed on whichever step it was found on, and a Job with
 * no plan has no step that works the groups to fall back to.
 */
export type CheckAt = { name: string; stepAttempt: number; step?: string };

/**
 * The Record row a Check's run is: `checked`, that name, on the step the caller
 * named or the one the groups are worked at, at that attempt. **Nothing where
 * the Record holds no such row.**
 */
function checkRowOf(rows: readonly LedgerRow[], detail: JobWhole | null, at: CheckAt): string | undefined {
  const step = at.step ?? (detail === null ? undefined : stepThatWorksTheGroups(detail));
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
  opensRow,
  trail,
}: RecordTabProps) {
  const [filter, setFilter] = useState<RecordFilter>("all");
  // Which step the rows are narrowed to. `null` is every step, and the Job's
  // own rows with them.
  const [step, setStep] = useState<string | null>(null);
  // Which row is open. **Held here and not in the ledger**, so a live redraw of
  // the Record does not close the row somebody is reading.
  const [openRow, setOpenRow] = useState<string | null>(() =>
    opensRow ?? (opensCheck === undefined ? null : (checkRowOf(rows, detail, opensCheck) ?? null)),
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
  useEffect(() => trail?.onHere(open === undefined ? null : { id: String(open.cursor), label: titleOf(open) }), [open?.cursor]);
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
          // Jumped to, Close goes back — `trail.ts`.
          onOpenRow={(id) => (id === null && trail?.close !== undefined ? trail.close() : setOpenRow(id))}
          kindMarks={filter === "all"}
          floor={floor}
          back={trail?.back}
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
