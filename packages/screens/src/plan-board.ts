// The plan board, composed — pure, so every sentence it draws is tested without
// a browser. `#1536`, `#1535`.
//
// **One board, because there was never more than one thing drawn.** A second
// one stood under the Workflow canvas drawing these same groups until the owner
// took them off it (28 Sep 2026). What it said that a plan alone does not —
// what a boundary came to, and what the failed Check was held to and got — is
// composed here, on the card the plan already had.
//
// **The sentences are `tab-plan-read.ts`'s**, which this imports and which
// imports nothing back: a second spelling of a group's state or a task's spend
// is the drift `lib/job-states.js` was deleted for.

import type {
  GroupBoundaryCheck,
  GroupBoundaryProps,
  PlanBoardGroup,
  PlanBoardProps,
  PlanMove,
  PlanBoardTask,
} from "@armada/components";
import type { JobDetail as JobWhole, StepDetail } from "@armada/protocol";

import type { CaseView } from "./draft/cases";
import type { JobDraft } from "./draft/held";
import type { GroupState, GroupView } from "./draft/group";
import type { TaskView } from "./draft/task";
import {
  besideSaid,
  caseReads,
  casesOf,
  costSaid,
  droppedSaid,
  failedChecksOf,
  groupSaid,
  groupsOf,
  markOf,
  overlapsOf,
  retrySaid,
  runBySaid,
  scopeRootOf,
  touchedByOf,
  turnsSaid,
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
 * boundary nothing reached says nothing rather than `0 failed`. **`all
 * passed`, with no number**: the bar beside it is already how many
 * (`design-system.md`, hard rule 7).
 */
export function verdictSaid(group: GroupView, failed: readonly string[]): string | undefined {
  if (group.state === "checking") return "running now";
  if (!hasRun(group.state)) return undefined;
  if (failed.length > 0) return `${failed.join(", ")} failed`;
  return group.checks_selected.length === 0 ? undefined : "all passed";
}

// What stopping a group forbids was a sentence here until 28 Sep, when the
// owner cut it: `No task of group 4 starts until this boundary passes.` is the
// ordering rule, true of a step that has never run, and guide 4 already says
// it twice over. Group 4's own row says it is waiting, which is this Job's
// fact and stays.

/**
 * The run a Check's row in the Record is, where the boundary has run: the
 * latest by that name on the step the groups are worked at, **and only where
 * it came to what the boundary reads**. A step's `check_runs` is one list for
 * every group in it, so a group still waiting would otherwise open an earlier
 * group's run, and a group that passed would open a later group's red.
 */
function runOf(step: StepDetail | undefined, name: string, reads: GroupBoundaryCheck["reads"]) {
  if (reads !== "passed" && reads !== "failed") return undefined;
  const runs = (step?.check_runs ?? []).filter((one) => one.name === name);
  const latest = Math.max(0, ...runs.map((one) => one.attempt));
  const run = runs.find((one) => one.attempt === latest);
  return run?.outcome === reads ? run : undefined;
}

/**
 * One Check at a boundary, and what a failed one was held to and got.
 *
 * **`expected` and `produced` are the run's, each under its own label** (the
 * owner, 29 Sep 2026: *why does this say every test passes but then says 1 of
 * 1384 failed?*). Run together with no labels they read as one claim. The
 * Check's output is a file, read on its own Record row, which the press opens.
 */
function checkOf(
  group: GroupView,
  name: string,
  failed: readonly string[],
  step: StepDetail | undefined,
  onOpenCheck: ((name: string, stepAttempt: number) => void) | undefined,
): GroupBoundaryCheck {
  const reads = checkReads(group, name, failed);
  const run = runOf(step, name, reads);
  const told = reads === "failed" ? run : undefined;
  return {
    name,
    reads,
    ...(told?.expected === undefined ? {} : { expected: sentenceCase(told.expected) }),
    ...(told?.produced === undefined ? {} : { result: sentenceCase(told.produced) }),
    ...(run === undefined || onOpenCheck === undefined
      ? {}
      : { onOpen: () => onOpenCheck(name, run.attempt) }),
  };
}

function sentenceCase(said: string): string {
  return said.charAt(0).toUpperCase() + said.slice(1);
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
  const turns = turnsSaid(task);
  const cost = costSaid(task);
  const later = touchedBy.get(task.id);
  return {
    id: task.id,
    title: task.title,
    mark: markOf(task.state),
    tier: task.tier,
    model: task.model,
    runBy: runBySaid(task),
    ...(turns === undefined ? {} : { turnsSays: turns }),
    ...(cost === undefined ? {} : { costSays: cost }),
    ...(beside === undefined ? {} : { besideSays: beside }),
    ...(later === undefined ? {} : { touchedSays: `touched later · ${later}` }),
    ...(task.failed_reason === undefined ? {} : { failedReason: task.failed_reason }),
  };
}

/**
 * One group's boundary — the Checks bar, and the tests region kept apart.
 *
 * `onOpenCheck` opens a Check's own Record row, by its name and the step
 * attempt that ran it. Absent, no Check is a button.
 */
export function boundaryOf(
  group: GroupView,
  cases: readonly CaseView[],
  whole: JobWhole | null,
  step: StepDetail | undefined,
  onOpenCheck?: (name: string, stepAttempt: number) => void,
): GroupBoundaryProps {
  const failed = failedChecksOf(whole, group);
  const checks = group.checks_selected.map((name) => checkOf(group, name, failed, step, onOpenCheck));
  const verdict = verdictSaid(group, failed);
  const retry = retrySaid(group.retry_count);
  const atBoundary = cases.filter((one) => one.groups.includes(group.id));
  // **What dropped a case, where one did.** `reads` is the whole of what a row
  // says here, so a case a scope revision took out has to say that in it or
  // the row reads as `dropped` with nothing naming who dropped it.
  const tests = atBoundary.map((one) => ({
    id: one.id,
    spec: one.spec,
    reads: droppedSaid(one) ?? caseReads(one),
  }));
  return {
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
    ...(tests.length === 0 ? {} : { tests }),
  };
}

/**
 * One group's card, sentences and all.
 *
 * **`step` is where a boundary's Check runs are read from.** Absent, no Check
 * carries what it got or opens a row — which is what a plan nobody has run
 * yet says.
 */
export function groupCardOf(
  group: GroupView,
  cases: readonly CaseView[],
  touchedBy: Map<string, string>,
  whole: JobWhole | null,
  step?: StepDetail,
  overlaps: readonly { says: string; paths: readonly string[] }[] = [],
  onOpenCheck?: (name: string, stepAttempt: number) => void,
): PlanBoardGroup {
  return {
    id: group.id,
    ordinal: group.ordinal,
    state: group.state,
    says: groupSaid(group.state),
    concurrent: group.concurrent,
    shapeSays: shapeSaid(group),
    scope: scopeRootOf(group.scope),
    tasks: group.tasks.map((task) => taskRowOf(task, touchedBy)),
    boundary: boundaryOf(group, cases, whole, step, onOpenCheck),
    ...(overlaps.length === 0 ? {} : { overlaps }),
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
  onOpenCheck?: (name: string, stepAttempt: number) => void,
): PlanBoardProps | undefined {
  const groups = groupsOf(whole, draft);
  if (groups.length === 0) return undefined;
  const cases = casesOf(whole, draft);
  const touchedBy = touchedByOf(groups);
  const overlaps = overlapsOf(groups);
  return {
    approach: whole?.work_plan?.approach ?? "",
    groups: groups.map((group) =>
      groupCardOf(
        group,
        cases,
        touchedBy,
        whole,
        step,
        overlaps.get(group.id) ?? [],
        onOpenCheck,
      ),
    ),
    ...(revisable ? { askable: true } : {}),
    ...(openTaskId === undefined ? {} : { openTaskId }),
    onOpenTask,
  };
}

/**
 * The groups with one move applied — where a person dropped a group or a
 * task, drawn while the move is out. **Ordinals stay the plan's**: a group is
 * named by the number it was planned with until Fleet says otherwise.
 */
export function movedGroups(groups: readonly PlanBoardGroup[], move: PlanMove): PlanBoardGroup[] {
  if (move.task === undefined) {
    const from = groups.findIndex((one) => one.id === move.group);
    if (from < 0) return [...groups];
    const rest = groups.filter((one) => one.id !== move.group);
    rest.splice(move.to, 0, groups[from]!);
    return rest;
  }
  const task = groups.flatMap((one) => one.tasks).find((one) => one.id === move.task);
  if (task === undefined) return [...groups];
  return groups.map((group) => {
    const tasks = group.tasks.filter((one) => one.id !== move.task);
    if (group.id === move.group) tasks.splice(move.to, 0, task);
    return tasks.length === group.tasks.length && group.id !== move.group ? group : { ...group, tasks };
  });
}
