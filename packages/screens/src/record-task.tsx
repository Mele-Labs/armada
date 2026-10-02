// A task row's reading: the action, why, and what supports it.

import { GroupBoundary, PathChip } from "@armada/components";
import type { JobDetail as JobWhole } from "@armada/protocol";

import { FieldLabel, Eyebrow } from "./regions";
import type { CaseView } from "./draft/cases";
import type { GroupView } from "./draft/group";
import type { LedgerRow } from "./draft/ledger";
import { boundaryOf } from "./plan-board";
import { touchedByOf } from "./tab-plan-read";
import { stepThatWorksTheGroups } from "./workflow-canvas";
import { basename, Field } from "./record-fields";

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
export function TaskRead({
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
      {(row.kind === "task_done" || row.kind === "task_handed_in") &&
      (task.expects !== undefined || task.shown !== undefined) ? (
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
