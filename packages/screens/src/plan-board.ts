// The plan board, composed — pure, so every sentence it draws is tested without
// a browser. `#1536`, `#1535`.
//
// **One board, because there was never more than one thing drawn.** A second
// one stood under the Workflow canvas drawing these same groups until the owner
// took them off it (28 Sep 2026). What it said that a plan alone does not —
// what a boundary came to, and the failed Check's own output handed on — is
// composed here, on the card the plan already had.
//
// **The sentences are `tab-plan-read.ts`'s**, which this imports and which
// imports nothing back: a second spelling of a group's state or a task's spend
// is the drift `lib/job-states.js` was deleted for.

import type {
  GroupBoundaryCheck,
  GroupBoundaryProps,
  PlanBoardAsk,
  PlanBoardGroup,
  PlanBoardProps,
  PlanBoardTask,
} from "@armada/components";
import type { JobDetail as JobWhole, StepDetail } from "@armada/protocol";

import type { CaseView } from "./draft/cases";
import type { JobDraft } from "./draft/held";
import type { GroupState, GroupView } from "./draft/group";
import type { TaskView } from "./draft/task";
import {
  asksOf,
  besideSaid,
  boundarySaid,
  caseReads,
  casesOf,
  clashesOf,
  droppedSaid,
  failedChecksOf,
  groupSaid,
  groupsOf,
  markOf,
  retrySaid,
  runBySaid,
  spentSaid,
  testsSaid,
  touchedByOf,
} from "./tab-plan-read";

/** Whether a group's boundary has already run. */
function hasRun(state: GroupState): boolean {
  return state === "passed" || state === "failed" || state === "retrying" || state === "landed";
}

/**
 * What one Check at this boundary reads as.
 *
 * **Attributed by the group's own state and never by the run alone.** A step's
 * `check_runs` is one list for every group in it, so a passed group would
 * otherwise take a later group's red.
 */
function checkReads(
  group: GroupView,
  name: string,
  failed: readonly string[],
): GroupBoundaryCheck["reads"] {
  if (failed.includes(name)) return "failed";
  if (group.state === "checking") return "running";
  return hasRun(group.state) ? "passed" : "not run";
}

/**
 * What the boundary came to, in words. **Absent until it has run** — a
 * boundary nothing reached says nothing rather than `0 failed`.
 */
export function verdictSaid(group: GroupView, failed: readonly string[]): string | undefined {
  if (group.state === "checking") return "running now";
  if (!hasRun(group.state)) return undefined;
  if (failed.length > 0) return `${failed.join(", ")} failed`;
  const passed = group.checks_selected.length;
  return passed === 0 ? undefined : `all ${passed} passed`;
}

// What stopping a group forbids was a sentence here until 28 Sep, when the
// owner cut it: `No task of group 4 starts until this boundary passes.` is the
// ordering rule, true of a step that has never run, and guide 4 already says
// it twice over. Group 4's own row says it is waiting, which is this Job's
// fact and stays.

/**
 * What the next Drone is told: the failed Check's own output, verbatim.
 *
 * **Never a summary.** A retry working from a paraphrase is a retry working
 * from something nobody can check against the run that produced it.
 */
export function toldNextOf(step: StepDetail | undefined, failed: readonly string[]): string | undefined {
  const run = (step?.check_runs ?? []).find((one) => failed.includes(one.name) && one.outcome === "failed");
  if (run === undefined) return undefined;
  const lines = [run.expected, run.produced].filter((one): one is string => one !== undefined);
  return lines.length === 0 ? undefined : lines.join("\n");
}

/**
 * `2 tasks, at the same time` — fan out — or `2 tasks, one after another`.
 *
 * **One fact, so one sentence.** The Job's Drone cap rode here behind a `·`
 * until 28 Sep, and `2 tasks, at the same time · this Job runs 2 Drones at
 * once` read as one number said twice. The cap is the Job's rather than this
 * group's, and Overview carries it as `Drones at once`.
 */
export function shapeSaid(group: GroupView): string {
  const many = `${group.tasks.length} ${group.tasks.length === 1 ? "task" : "tasks"}`;
  if (group.tasks.length === 1) return `${many}, on its own`;
  if (!group.concurrent) return `${many}, one after another`;
  return `${many}, at the same time`;
}

/**
 * One task's row.
 *
 * **The tier, the model it resolved to and how it is run are three fields, not
 * one line.** The board joins them; a caller that joined them first would be
 * composing prose, which is `PlanBoard`'s own rule.
 */
function taskRowOf(task: TaskView, touchedBy: Map<string, string>): PlanBoardTask {
  const beside = besideSaid(task);
  const spent = spentSaid(task);
  const later = touchedBy.get(task.id);
  return {
    id: task.id,
    title: task.title,
    mark: markOf(task.state),
    tier: task.tier,
    model: task.model,
    runBy: runBySaid(task),
    ...(spent === undefined ? {} : { spentSays: spent }),
    ...(beside === undefined ? {} : { besideSays: beside }),
    ...(later === undefined ? {} : { touchedSays: `touched later · ${later}` }),
    ...(task.failed_reason === undefined ? {} : { failedReason: task.failed_reason }),
  };
}

/** One group's boundary — the Checks bar, and the tests region kept apart. */
export function boundaryOf(
  group: GroupView,
  cases: readonly CaseView[],
  whole: JobWhole | null,
  step: StepDetail | undefined,
): GroupBoundaryProps {
  const failed = failedChecksOf(whole, group);
  const checks = group.checks_selected.map((name) => ({
    name,
    reads: checkReads(group, name, failed),
  }));
  const verdict = verdictSaid(group, failed);
  const retry = retrySaid(group.retry_count);
  const told = toldNextOf(step, failed);
  const atBoundary = cases.filter((one) => one.groups.includes(group.id));
  // **What dropped a case, where one did.** `reads` is the whole of what a row
  // says here, so a case a scope revision took out has to say that in it or
  // the row reads as `dropped` with nothing naming who dropped it.
  const tests = atBoundary.map((one) => ({
    id: one.id,
    spec: one.spec,
    reads: droppedSaid(one) ?? caseReads(one),
  }));
  const testsSay = testsSaid(group.state, tests.length);
  return {
    says: boundarySaid(group.state, checks.length),
    checks,
    checksAbsent: "No Check runs at this group's end.",
    ...(verdict === undefined ? {} : { verdictSays: verdict }),
    ...(failed.length > 0
      ? { verdictNamed: "failed" as const }
      : hasRun(group.state)
        ? { verdictNamed: "passed" as const }
        : {}),
    ...(retry === undefined ? {} : { retrySays: retry }),
    ...(group.commit === undefined ? {} : { commit: group.commit }),
    ...(told === undefined ? {} : { toldNext: told }),
    ...(testsSay === undefined ? {} : { testsSay }),
    ...(tests.length === 0 ? {} : { tests }),
  };
}

/**
 * One group's card, sentences and all.
 *
 * **`step` is only where a failed Check's output is read from.** Absent, the
 * boundary hands nothing on — which is what a plan nobody has run yet says.
 */
export function groupCardOf(
  group: GroupView,
  cases: readonly CaseView[],
  touchedBy: Map<string, string>,
  whole: JobWhole | null,
  asks: readonly PlanBoardAsk[] = [],
  step?: StepDetail,
): PlanBoardGroup {
  return {
    id: group.id,
    ordinal: group.ordinal,
    state: group.state,
    says: groupSaid(group.state),
    shapeSays: shapeSaid(group),
    scope: group.scope,
    tasks: group.tasks.map((task) => taskRowOf(task, touchedBy)),
    boundary: boundaryOf(group, cases, whole, step),
    ...(asks.length === 0 ? {} : { asks }),
  };
}

/**
 * The whole board, from a Job and whatever draft the moment carries.
 *
 * `revisable` is whether the plan may still be asked about — the step that
 * recorded it is waiting on a person, and nothing else is out. **A plan past
 * its gate draws no controls at all**: it is a record then, and a control that
 * sends an ask nobody will answer is worse than none.
 */
export function planBoardOf(
  whole: JobWhole | null,
  draft: JobDraft | undefined,
  onOpenTask: (taskId: string) => void,
  openTaskId?: string,
  revisable = false,
  step?: StepDetail,
): PlanBoardProps | undefined {
  const groups = groupsOf(whole, draft);
  if (groups.length === 0) return undefined;
  const cases = casesOf(whole, draft);
  const touchedBy = touchedByOf(groups);
  const clashes = clashesOf(groups);
  return {
    approach: whole?.work_plan?.approach ?? "",
    groups: groups.map((group, at) =>
      groupCardOf(group, cases, touchedBy, whole, revisable ? asksOf(groups, at) : [], step),
    ),
    ...(clashes.length === 0 ? {} : { clashes }),
    ...(revisable ? { askable: true } : {}),
    ...(openTaskId === undefined ? {} : { openTaskId }),
    onOpenTask,
  };
}
