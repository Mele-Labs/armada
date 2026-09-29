// The open Record row, read whole: the head every row shares, and the reading
// its kind takes under it.
//
// **One module per reading.** A Check's is `record-check.tsx`, a task's
// `record-task.tsx`, a File's diff `record-file.tsx`, the Drone's
// `record-drone.tsx`, and every other row's values `record-fields.tsx`. This
// file picks among them; `tab-record.tsx` holds which row is open.

import { useEffect } from "react";
import { ChevronRight } from "lucide-react";
import { Button, type JobLedgerRow } from "@armada/components";
import type { Diff, JobDetail as JobWhole } from "@armada/protocol";

import { absoluteOf, clock } from "./duration";
import { rowSays, statusOf } from "./record";
import type { useCheckOutputs } from "./outputs";
import { Eyebrow } from "./regions";
import { titleOf } from "./record-cells";
import type { CaseView } from "./draft/cases";
import { taskGroupsOf, type GroupView } from "./draft/group";
import { familyOf, pathsOf, type LedgerRow } from "./draft/ledger";
import { droneOfTask } from "./tab-plan-read";
import { CheckRunRead } from "./record-check";
import { DroneRead } from "./record-drone";
import { RowFields } from "./record-fields";
import { FileDiff } from "./record-file";
import { TaskRead } from "./record-task";

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
 * One row, read whole.
 *
 * **A Check shows its output and what it stopped**, which is the reading the
 * four views could not put side by side: the run, the lines it printed, and
 * the step the run decided.
 */
export function RowRead({
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
  const family = familyOf(row.kind);
  const status = statusOf(row);
  // The task a task row is about, read off the plan rather than the row: the
  // row carries the short reason, and the task carries the rest.
  const task =
    family === "tasks" && row.coord?.task !== undefined
      ? tasks.find((one) => one.id === row.coord?.task)
      : undefined;
  const diffPaths = family === "files" ? pathsOf(row) : (task?.scope ?? []);

  // A File row asks for the Job's diff when it opens, and never the Record:
  // the patch is the expensive read, and Overview lets it go on the way out.
  // A task row asks for it too, for its own files' part.
  const readsDiff = diffPaths.length > 0;
  useEffect(() => {
    if (!readsDiff) return;
    onReadDiff(jobId);
    return () => onReadDiff(null);
  }, [readsDiff, jobId]);

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
          <CheckRunRead
            row={row}
            rows={rows}
            step={step}
            run={run}
            detail={detail}
            groups={groups}
            outputs={outputs}
            onOpenStep={onOpenStep}
            onOpenTask={onOpenTask}
            taskRowOf={taskRow}
          />
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
