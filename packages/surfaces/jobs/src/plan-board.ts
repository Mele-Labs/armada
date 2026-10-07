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
import { GUIDE_GROUP_BOUNDARY } from "@armada/components";
import type { CheckRun, CheckUnderway, JobDetail as JobWhole, MovePlan, StepDetail } from "@armada/protocol";

import type { JobCheckLog } from "./check-log-sheet";
import type { CaseView } from "./draft/cases";
import { basename } from "./phases";
import type { JobDraft } from "./draft/held";
import type { GroupState, GroupView } from "./draft/group";
import type { TaskView } from "./draft/task";
import {
  awaitingSaid,
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
 * Whether the step's gate, running now, is this group's.
 *
 * **Fleet never serves a group as `checking`** — it draws that state from the
 * step's own `checking` (`GroupState` in `plan_group.rs`) — so a served group
 * stays `running` while its Checks run, and reading only the state left every
 * Check `not run` beside a step panel showing six passed (owner, 5 Oct 2026).
 * `checking` names no group, and groups run one at a time, so the gate is the
 * group in flight's. **Not another's**: a finished row naming a different group
 * (#1652) means this gate is not this one's, and a group that has not begun
 * takes none.
 *
 * **A group run again is `retrying` while its gate runs**, so that state holds
 * the gate too: reading only `running` left the second run's boundary on the
 * first run's rows (owner, 7 Oct 2026).
 */
function gateIsHere(group: GroupView, step: StepDetail | undefined): boolean {
  if (group.state === "checking") return true;
  if (group.state !== "running" && group.state !== "joining" && group.state !== "retrying") return false;
  const gate = step?.checking;
  if (gate === undefined) return false;
  return gate.checks.every((one) => one.ran?.group === undefined || one.ran.group === group.id);
}

/**
 * What one Check at this boundary reads as.
 *
 * **Attributed by the group's own state and never by the run alone.** A step's
 * `check_runs` is one list for every group in it, so a passed group would
 * otherwise take a later group's red.
 *
 * **A group whose gate is running reads each Check off the gate**, which is
 * this boundary's own while it runs: waiting is not run, started is running,
 * finished is what it came to. Every one read `running` until a segment became
 * a press (2 Oct 2026), and a running segment that opens nothing, because its
 * Check has not started, says two things at once.
 */
function checkReads(
  group: GroupView,
  name: string,
  failed: readonly string[],
  step: StepDetail | undefined,
  kept: readonly CheckRun[] | undefined,
): { reads: GroupBoundaryCheck["reads"]; run?: CheckRun } {
  // **One run decides every row**: the live gate's while one is going,
  // otherwise the group's latest finished run. The step's earlier red is
  // never read beside a later run's rows.
  if (gateIsHere(group, step)) return underwayReads(step?.checking?.checks.find((one) => one.name === name));
  if (kept !== undefined) {
    const run = kept.find((one) => one.name === name);
    return run === undefined ? { reads: "not run" } : { reads: ranReads(run), run };
  }
  if (failed.includes(name)) return { reads: "failed" };
  return { reads: hasRun(group.state) ? "passed" : "not run" };
}

/**
 * What a finished Check's outcome reads as. **`skipped` is its own reading**
 * (owner, 7 Oct 2026): it advances the step and measured nothing, so it is
 * neither a pass nor a failure. Every other outcome that did not pass is red.
 */
function ranReads(run: CheckRun): GroupBoundaryCheck["reads"] {
  if (run.outcome === "passed") return "passed";
  return run.outcome === "skipped" ? "skipped" : "failed";
}

/** One Check the gate holds, read. Absent from it reads as the gate running, as before. */
function underwayReads(live: CheckUnderway | undefined): { reads: GroupBoundaryCheck["reads"]; run?: CheckRun } {
  if (live === undefined) return { reads: "running" };
  if (live.ran !== undefined) return { reads: ranReads(live.ran), run: live.ran };
  return { reads: live.started_at === undefined ? "not run" : "running" };
}

/**
 * The group's latest finished run of the step's Checks, by the run Fleet
 * records it under (#1652). **Absent where Fleet names no group's rows** — a
 * Fleet before 23.4, or a group that has not run — and the group's state then
 * speaks, as it did.
 */
function keptRunOf(step: StepDetail | undefined, group: GroupView): readonly CheckRun[] | undefined {
  if (!hasRun(group.state)) return undefined;
  const own = (step?.check_runs ?? []).filter((one) => one.group === group.id);
  if (own.length === 0) return undefined;
  const latest = Math.max(...own.map((one) => one.group_attempt ?? 0));
  return own.filter((one) => (one.group_attempt ?? 0) === latest);
}

/**
 * What the boundary came to, in words. **Absent until it has run** — a
 * boundary nothing reached says nothing rather than `0 failed`. **`all
 * passed`, with no number**: the bar beside it is already how many
 * (`design-system.md`, hard rule 7). **`all passed` only where nothing was
 * skipped**: a skipped Check did not pass, so a run holding one says `none
 * failed`, and a run that skipped every Check says nothing of passing.
 */
export function verdictSaid(
  group: GroupView,
  failed: readonly string[],
  step?: StepDetail,
  reads: readonly GroupBoundaryCheck["reads"][] = [],
): string | undefined {
  if (gateIsHere(group, step)) return "running now";
  if (!hasRun(group.state)) return waitingSaid(group);
  if (failed.length > 0) return `${failed.join(", ")} failed`;
  if (group.checks_selected.length === 0) return undefined;
  if (reads.length > 0 && reads.every((one) => one === "skipped")) return undefined;
  return reads.includes("skipped") ? "none failed" : "all passed";
}

/**
 * What a running group's Checks wait on: the tasks not yet in. **A group's gate
 * runs at its end**, once no task is open or working (`docs/concepts/plan.md`,
 * *A Drone per task*), so until then `not run` is a wait and not a skip. A
 * group that has not begun says nothing: all of it would be waiting.
 */
function waitingSaid(group: GroupView): string | undefined {
  if (group.state !== "running" && group.state !== "joining") return undefined;
  const out = group.tasks.filter((task) => task.state === "open" || task.state === "working");
  return out.length === 0 ? undefined : `Waiting on ${out.map((task) => task.id).join(", ")}`;
}

// What stopping a group forbids was a sentence here until 28 Sep, when the
// owner cut it: `No task of group 4 starts until this boundary passes.` is the
// ordering rule, true of a step that has never run, and guide 4 already says
// it twice over. Group 4's own row says it is waiting, which is this Job's
// fact and stays.

/**
 * The run a Check's row in the Record is, where the boundary has run.
 *
 * **The group's own latest run, where Fleet records the group** (#1652): a
 * passed group opens its own, whatever a later group's run of that Check said.
 * A Fleet before 23.4 named no group, and there it is the latest by that name
 * on the step, **only where it came to what the boundary reads** — a group
 * still waiting would otherwise open an earlier group's run, and a group that
 * passed a later group's red.
 */
function runOf(
  step: StepDetail | undefined,
  name: string,
  reads: GroupBoundaryCheck["reads"],
  group: GroupView,
) {
  if (reads !== "passed" && reads !== "failed") return undefined;
  const runs = (step?.check_runs ?? []).filter((one) => one.name === name);
  const own = runs.filter((one) => one.group === group.id);
  if (own.length > 0) {
    const latest = Math.max(...own.map((one) => one.group_attempt ?? 0));
    return own.find((one) => (one.group_attempt ?? 0) === latest);
  }
  if (runs.some((one) => one.group !== undefined)) return undefined;
  const latest = Math.max(0, ...runs.map((one) => one.attempt));
  const run = runs.find((one) => one.attempt === latest);
  return run?.outcome === reads ? run : undefined;
}

/**
 * Where a Check at this boundary keeps its log, or `undefined` where it has
 * none to read.
 *
 * **While the group checks, the gate's live log**, of a Check that has started
 * whether or not it has ended: the socket sends an ended one whole and closes.
 * **Once the boundary has run, the kept file of the run it reads**, `runOf`'s. A Check that has not started has no log, so its
 * segment opens nothing rather than an empty panel.
 */
function logOf(
  group: GroupView,
  step: StepDetail | undefined,
  name: string,
  run: ReturnType<typeof runOf>,
): JobCheckLog | undefined {
  if (gateIsHere(group, step)) {
    const live = step?.checking?.checks.find((one) => one.name === name)?.output_path;
    return live === undefined ? undefined : { name, kept: basename(live), live: true };
  }
  const kept = run?.output_path;
  return kept === undefined || run === undefined
    ? undefined
    : {
        name,
        kept: basename(kept),
        live: false,
        stepAttempt: run.attempt,
        // The group's own row on the Record, where Fleet names it (#1652).
        ...(run.group === undefined ? {} : { group: run.group }),
      };
}

/**
 * One Check at a boundary, and what a failed one was held to and got.
 *
 * **`expected` and `produced` are the run's, each under its own label** (the
 * owner, 29 Sep 2026: *why does this say every test passes but then says 1 of
 * 1384 failed?*). Run together with no labels they read as one claim. The
 * Check's output is a file, which the press opens in the log panel.
 */
function checkOf(
  group: GroupView,
  name: string,
  failed: readonly string[],
  step: StepDetail | undefined,
  kept: readonly CheckRun[] | undefined,
  onOpenCheckLog: ((log: JobCheckLog) => void) | undefined,
): GroupBoundaryCheck {
  const { reads, run: given } = checkReads(group, name, failed, step, kept);
  // The step's runs are an earlier group's while this one is still checking.
  const run = gateIsHere(group, step) ? undefined : (given ?? runOf(step, name, reads, group));
  const told = reads === "failed" ? (given ?? run) : undefined;
  const log = logOf(group, step, name, run);
  return {
    name,
    reads,
    ...(reads === "skipped" && given?.produced !== undefined ? { why: given.produced } : {}),
    ...(told?.expected === undefined ? {} : { expected: sentenceCase(told.expected) }),
    ...(told?.produced === undefined ? {} : { result: sentenceCase(told.produced) }),
    ...(log === undefined || onOpenCheckLog === undefined ? {} : { onOpen: () => onOpenCheckLog(log) }),
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
/**
 * What running at once cannot see: two tasks whose edit calls named one file
 * run again in turn, and a write made through the shell names none (answer 10).
 */
export const SHELL_UNSEEN =
  "Two that edit one file run again one after the other; a write through the shell isn't seen";

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
  const awaiting = awaitingSaid(task.state);
  return {
    id: task.id,
    title: task.title,
    mark: markOf(task.state),
    ...(awaiting === undefined ? {} : { statusSays: awaiting }),
    ...(task.tier === undefined ? {} : { tier: task.tier }),
    ...(task.model === undefined ? {} : { model: task.model }),
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
 * `onOpenCheckLog` opens a Check's log in the log panel. Absent, no Check is a
 * button.
 */
export function boundaryOf(
  group: GroupView,
  cases: readonly CaseView[],
  whole: JobWhole | null,
  step: StepDetail | undefined,
  onOpenCheckLog?: (log: JobCheckLog) => void,
): GroupBoundaryProps {
  const live = gateIsHere(group, step);
  const kept = live ? undefined : keptRunOf(step, group);
  // **The run's own reds where one run is read**, and Fleet's older attribution only
  // where Fleet names no group's rows.
  const legacy = live || kept !== undefined ? [] : failedChecksOf(whole, group);
  const checks = group.checks_selected.map((name) => checkOf(group, name, legacy, step, kept, onOpenCheckLog));
  const failed = live
    ? checks.filter((one) => one.reads === "failed").map((one) => one.name)
    : kept === undefined
      ? legacy
      : kept.filter((one) => ranReads(one) === "failed").map((one) => one.name);
  const readings = checks.map((one) => one.reads);
  const verdict = verdictSaid(group, failed, step, readings);
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
    // What a boundary is: the one Armada word on the strip, so the one mark.
    guide: GUIDE_GROUP_BOUNDARY,
    checksAbsent: "No Check runs at this group's end.",
    ...(verdict === undefined ? {} : { verdictSays: verdict }),
    ...(failed.length > 0
      ? { verdictNamed: "failed" as const }
      : hasRun(group.state) && !live && !(readings.length > 0 && readings.every((one) => one === "skipped"))
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
  onOpenCheckLog?: (log: JobCheckLog) => void,
): PlanBoardGroup {
  return {
    id: group.id,
    ordinal: group.ordinal,
    state: group.state,
    says: groupSaid(group.state),
    concurrent: group.concurrent,
    shapeSays: shapeSaid(group),
    ...(group.concurrent ? { shapeHint: SHELL_UNSEEN } : {}),
    scope: scopeRootOf(group.scope),
    tasks: group.tasks.map((task) => taskRowOf(task, touchedBy)),
    boundary: boundaryOf(group, cases, whole, step, onOpenCheckLog),
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
  onOpenCheckLog?: (log: JobCheckLog) => void,
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
        onOpenCheckLog,
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

/**
 * A drop as Fleet takes it: placed after the task or the group it now follows,
 * or first where it follows none. **By `after`, never by index** — spike 022's
 * bodies table: an index counted at the drag is stale the moment a Drone adds or
 * drops a task.
 */
export function moveSent(groups: readonly PlanBoardGroup[], move: PlanMove): MovePlan {
  const moved = movedGroups(groups, move);
  if (move.task === undefined) {
    const before = moved[moved.findIndex((one) => one.id === move.group) - 1];
    return before === undefined ? { group: move.group } : { group: move.group, after: before.id };
  }
  const tasks = moved.find((one) => one.id === move.group)?.tasks ?? [];
  const before = tasks[tasks.findIndex((one) => one.id === move.task) - 1];
  return before === undefined
    ? { group: move.group, task: move.task }
    : { group: move.group, task: move.task, after: before.id };
}
