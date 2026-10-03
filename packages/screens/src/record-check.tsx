// A Check row's reading: what the run produced and was held to, the lines it
// printed, and what it stopped — the step, and the group the boundary card
// names with it.

import { useEffect } from "react";
import { CHECK_ADVANCES, ConsoleOutput, RowLink, TaskMark, WorkflowStepCard } from "@armada/components";
import type { JobDetail as JobWhole } from "@armada/protocol";

import { clock } from "./duration";
import { noteFor, regionOf, rowsOf, type useCheckOutputs } from "./outputs";
import { Eyebrow, FieldLabel } from "./regions";
import type { GroupView } from "./draft/group";
import type { LedgerRow } from "./draft/ledger";
import { failedChecksOf, markOf } from "./tab-plan-read";
import { stepNodeId, workflowRunOf } from "./workflow-canvas";
import { basename, sentenceCase } from "./record-fields";

/**
 * The group whose boundary a failed Check's run held back.
 *
 * **The group the run names, where Fleet records it** (#1652). A Fleet before
 * 23.4 named none, and there it is `failedChecksOf`'s own attribution, the one
 * the boundary card draws: the group carrying a failure that runs this Check,
 * read off the working step's latest attempt.
 */
function groupHeldBy(
  detail: JobWhole,
  groups: readonly GroupView[],
  step: JobWhole["steps"][number],
  run: JobWhole["steps"][number]["check_runs"][number],
): GroupView | undefined {
  if (run.group !== undefined) return groups.find((group) => group.id === run.group);
  if (step.step_id !== detail.job.current_step_id) return undefined;
  const latest = Math.max(0, ...step.check_runs.map((one) => one.attempt));
  if (run.attempt !== latest) return undefined;
  return groups.find(
    (group) => group.checks_selected.includes(run.name) && failedChecksOf(detail, group).includes(run.name),
  );
}

/**
 * A Check's run, read whole: **its output and what it stopped**, which is the
 * reading the four views could not put side by side.
 */
export function CheckRunRead({
  row,
  rows,
  step,
  run,
  detail,
  groups,
  outputs,
  onOpenStep,
  onOpenTask,
  taskRowOf: taskRow,
}: {
  row: LedgerRow;
  /** Every row of the Record, so a failed Check can say whether a later run passed. */
  rows: readonly LedgerRow[];
  step: JobWhole["steps"][number] | undefined;
  run: JobWhole["steps"][number]["check_runs"][number];
  detail: JobWhole;
  groups: readonly GroupView[];
  outputs: ReturnType<typeof useCheckOutputs>;
  onOpenStep: (stepId: string) => void;
  onOpenTask: (taskId: string) => void;
  /** A task's Record row, where it has one — which task lines are pressable. */
  taskRowOf: (taskId: string) => string | undefined;
}) {
  const kept = run.output_path;
  const name = kept === undefined ? undefined : basename(kept);
  const held = name === undefined ? undefined : outputs.of(name);

  // Opening the row is the ask: the reading is what the pane is for, and a
  // second press to see it was the step the board does not draw.
  useEffect(() => {
    if (name !== undefined && outputs.of(name) === undefined) outputs.fetch(name);
  }, [name, outputs]);

  const passedLater = run.outcome === "failed" ? laterPass(row, rows) : undefined;
  // The group the run held back, where the boundary card names one: a Check
  // that stopped its step stopped that group's tasks with it (the owner, 29 Sep
  // 2026: *not just the step from completing but a task in the plan*).
  const heldGroup =
    step === undefined || CHECK_ADVANCES[run.outcome] !== false
      ? undefined
      : groupHeldBy(detail, groups, step, run);
  // The step's node, as the Workflow canvas draws it — `workflowRunOf`'s own
  // card, so the two cannot say different things about one step.
  const node =
    step === undefined || CHECK_ADVANCES[run.outcome] !== false
      ? undefined
      : workflowRunOf({ whole: detail, groups, onOpen: () => onOpenStep(step.step_id) }).nodes.find(
          (one) => one.id === stepNodeId(step.step_id),
        )?.card;

  return (
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
      one.outcome.toLowerCase().startsWith("passed"),
  );
}
