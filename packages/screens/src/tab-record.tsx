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
import {
  ADVANCE_GATE,
  Button,
  ConsoleOutput,
  GUIDE_DRIFT,
  GuideMark,
  JobLedger,
  Select,
  STEP_STATE,
  type JobLedgerRow,
} from "@armada/components";
import type { JobDetail as JobWhole } from "@armada/protocol";
import { useNarrow } from "@armada/shell";

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
import { Eyebrow } from "./regions";
import { taskGroupsOf } from "./draft/group";
import { familyOf, type LedgerRow } from "./draft/ledger";

export type RecordTabProps = {
  jobId: string;
  /** The Job whole, where Fleet answered for it. */
  detail: JobWhole | null;
  /** Every row of the Record, newest first. */
  rows: readonly LedgerRow[];
  /** The window is at `--window-floor`. */
  floor: boolean;
  onReadCheckOutput: ReadCheckOutput;
  /** Say a sentence to the person — what an act that is not built yet answers. */
  onSaid: (sentence: string) => void;
};

export function RecordTab({
  jobId,
  detail,
  rows,
  floor,
  onReadCheckOutput,
  onSaid,
}: RecordTabProps) {
  const [filter, setFilter] = useState<RecordFilter>("all");
  // Which step the rows are narrowed to. `null` is every step, and the Job's
  // own rows with them.
  const [step, setStep] = useState<string | null>(null);
  // Which row is open. **Held here and not in the ledger**, so a live redraw of
  // the Record does not close the row somebody is reading.
  const [openRow, setOpenRow] = useState<string | null>(null);
  const narrow = useNarrow();

  const outputs = useCheckOutputs(onReadCheckOutput, jobId);

  const atStep = useMemo(() => underStep(rows, step), [rows, step]);
  const shown = useMemo(() => underFilter(atStep, filter), [atStep, filter]);
  const drawn = useMemo(() => ledgerRowsFor(shown, detail), [shown, detail]);
  // Beside the table the pane always reads a row, the newest until somebody
  // presses another: a column saying nothing is open is an empty frame. Folded
  // into a sheet, nothing opens until it is pressed.
  const reading = openRow ?? (narrow ? null : (drawn[0]?.id ?? null));
  const open = shown.find((row) => String(row.cursor) === reading);
  const openDrawn = drawn.find((row) => row.id === reading);
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
              <Select
                aria-label="Step"
                value={step ?? ""}
                onChange={(event) => {
                  setStep(event.target.value === "" ? null : event.target.value);
                  setOpenRow(null);
                }}
              >
                <option value="">Any step</option>
                {steps.map((one) => (
                  <option key={one.step_id} value={one.step_id}>
                    {one.label}
                  </option>
                ))}
              </Select>
            )
          }
          openRow={reading}
          onOpenRow={setOpenRow}
          kindMarks={filter === "all"}
          floor={floor}
          narrow={narrow}
          emptyNote={EMPTY[filter]}
          {...(open === undefined ? {} : { inspectorTitle: open.what })}
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
                    onSaid={onSaid}
                  />
                ),
              })}
        />
      )}
    </div>
  );
}

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
  onSaid: (sentence: string) => void;
};

/**
 * One row, read whole.
 *
 * **A Check shows its output and what it stopped**, which is the reading the
 * four views could not put side by side: the run, the lines it printed, and
 * the step the run decided.
 */
function RowRead({ row, rows, drawn, detail, outputs, onSaid }: RowReadProps) {
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
  const machine = family === "checks" || family === "files";

  // Opening the row is the ask: the reading is what the pane is for, and a
  // second press to see it was the step the board does not draw.
  useEffect(() => {
    if (name !== undefined && outputs.of(name) === undefined) outputs.fetch(name);
  }, [name, outputs]);

  const passedLater = run?.outcome === "failed" ? laterPass(row, rows) : undefined;

  return (
    <div className="armada-ledger__read">
      <header className="armada-ledger__read-head">
        <Eyebrow>
          {family == null ? drawn.where : `${FAMILY_SAYS[family]} · ${drawn.where}`}
        </Eyebrow>
        <p className="armada-ledger__read-name" data-machine={machine || undefined}>
          {row.what}
        </p>
        <p className="armada-ledger__read-status">
          {run === undefined ? null : (
            <span className="armada-ledger__read-outcome" data-tone={drawn.tone}>
              {sentenceCase(run.outcome)}
            </span>
          )}
          <span className="armada-ledger__read-meta" title={absoluteOf(row.at) ?? undefined}>
            {clock(row.at)} · {drawn.whoSays}
          </span>
        </p>
      </header>

      <div className="armada-ledger__read-body">
        {run === undefined ? (
          row.outcome === "" ? null : (
            <p className="armada-ledger__read-said" data-inline>
              {row.outcome}{" "}
              {/* What a file nobody declared costs the Job is the question the
                  row's words cannot answer without teaching. #1537, the owner,
                  28 Sep. */}
              {family === "files" ? <GuideMark guide={GUIDE_DRIFT} /> : null}
            </p>
          )
        ) : (
          <>
            {run.produced === undefined && run.expected === undefined ? null : (
              <p className="armada-ledger__read-said">
                {run.produced === undefined ? null : sentenceCase(run.produced)}
                {run.expected === undefined ? null : (
                  <span className="armada-ledger__read-expected">Expected {run.expected}</span>
                )}
              </p>
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

            {step === undefined ? null : (
              <section className="armada-ledger__read-section">
                <Eyebrow>What it stopped</Eyebrow>
                <div className="armada-ledger__read-well">
                  <p className="armada-ledger__read-said">
                    It gated {step.label}, which is now {STEP_STATE[step.state]?.verb ?? step.state}.
                    {step.advance_gate === undefined
                      ? ""
                      : ` It advances when ${ADVANCE_GATE[step.advance_gate]?.verb ?? step.advance_gate}.`}
                  </p>
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

        <RunAgain row={row} detail={detail} onSaid={onSaid} />
      </div>
    </div>
  );
}

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
      one.outcome.startsWith("passed"),
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
