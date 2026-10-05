// What the Workflow tab's inspector reads for the step, group or task a person
// has open. `#1539`.
//
// **What it cannot answer, it says.** The cases a group owes are a draft shape
// Fleet does not serve (`draft/group.ts`, `cases_at_boundary`), so the tests
// region draws the reason rather than an empty list that reads as "none owed".
//
// **A step that makes or works the plan links to it, and lists none of its
// tasks** (owner, 28 and 29 Sep 2026, `25i2` and `nm0h`).

import type {
  WorkflowInspectorCheck,
  WorkflowInspectorProps,
  WorkflowInspectorTask,
} from "@armada/components";
import { issueLink, type JobDetail as JobWhole, type StepDetail } from "@armada/protocol";

import { span } from "./duration";
import type { JobCheckLog } from "./check-log-sheet";
import { checksOf as gateChecksOf, isRunning, type CheckRead } from "./gates";
import { basename } from "./phases";
import type { GroupView } from "./draft/group";
import type { DroneView } from "./draft/drone";
import { trackOf } from "./step-phase";
import { ordered } from "./facts";
import { frozenBeneath } from "./frozen";
import { activityOf, stateOf } from "./run";
import { groupNodeId } from "./plan-canvas";
import { placeOf, stepNodeId, stepTheGroupsWereMadeAt } from "./workflow-canvas";

/** What the inspector draws, less the two controls the tab wires itself. */
export type WorkflowReading = Omit<WorkflowInspectorProps, "redirect" | "stop"> & {
  /** The Drones a redirect from here could reach, in the order they are listed. */
  drones: { id: string; label: string }[];
};

// Why there are no cases here said *Fleet does not serve the cases a boundary
// owes yet* until 28 Sep, and the owner asked what was missing and how it gets
// fixed (`frpl`). What is missing is coverage: nothing says which tests cover
// which files, so nothing can choose the ones a boundary owes. #1274 is the
// issue that decides it, by a `COVERS` file per spec folder.
const NO_CASES = {
  says: "No tests run here yet. Nothing tells Armada which tests cover these files, so it can't choose any. A COVERS file beside the tests would fix that.",
  issue: 1274,
} as const;

/**
 * Each Check the step declares, beside what it came to — **or where the gate
 * has it right now**, while the gate runs them (owner, 29 Sep 2026, `juhq`).
 * `gates.ts` is what reads the live set over the last ruling.
 */
function checksOf(
  step: StepDetail,
  now: number,
  only?: readonly string[],
  onOpenCheckLog?: (log: JobCheckLog) => void,
): WorkflowInspectorCheck[] {
  // **De-duplicated.** A step can declare the same Check twice under different
  // globs — the arc's `implement` declares `test` for Rust and again for
  // Bridge — and one command is one row whatever selected it.
  const reads = gateChecksOf(step);
  return reads
    .filter((read, at) => reads.findIndex((one) => one.name === read.name) === at)
    .filter((read) => only === undefined || only.includes(read.name))
    .map((read) => {
      const row: WorkflowInspectorCheck = { name: read.name };
      const log = onOpenCheckLog === undefined ? undefined : logOf(read);
      if (log !== undefined) row.onOpen = () => onOpenCheckLog!(log);
      if (isRunning(read)) {
        row.live = "running";
        const lasted = span(read.live!.started_at!, now);
        if (lasted !== null) row.outcome = lasted;
        return row;
      }
      if (read.live !== undefined && read.run === undefined) {
        row.live = "waiting";
        row.outcome = "waiting";
        return row;
      }
      const outcome = read.run?.outcome;
      if (outcome !== undefined) {
        row.outcome = outcome;
        if (outcome === "passed" || outcome === "failed") row.named = outcome;
      }
      return row;
    });
}

/**
 * Where a Check on this step keeps its log, or `undefined` where it has none.
 *
 * **While the gate holds the Check, its live log** — of one that has started,
 * ended or not, as `plan-board.ts` reads it. **Once the gate has ruled, the kept
 * file of the run.** A Check that is waiting, or never ran, has no log.
 */
function logOf(read: CheckRead): JobCheckLog | undefined {
  if (read.live !== undefined) {
    const started = read.live.started_at !== undefined || read.live.ran !== undefined;
    const path = read.live.output_path;
    return !started || path === undefined ? undefined : { name: read.name, kept: basename(path), live: true };
  }
  const kept = read.run?.output_path;
  return kept === undefined || read.run === undefined
    ? undefined
    : {
        name: read.name,
        kept: basename(kept),
        live: false,
        stepAttempt: read.run.attempt,
        ...(read.run.group === undefined ? {} : { group: read.run.group }),
      };
}

/**
 * What a task's own agent has spent: its turns, and its cost once it stopped.
 * The Record's Drone section reads the same two facts, from here.
 */
export function spentOf(task: Pick<GroupView["tasks"][number], "turns" | "cost_micros">): string[] {
  const facts: string[] = [];
  if (task.turns !== undefined) facts.push(`${task.turns} turns`);
  if (task.cost_micros !== undefined) facts.push(`$${(task.cost_micros / 1_000_000).toFixed(2)}`);
  return facts;
}

/** One task, with what it has spent where its own agent has stopped. */
function taskOf(task: GroupView["tasks"][number]): WorkflowInspectorTask {
  const facts: string[] = [];
  if (task.scope.length > 0) facts.push(`${task.scope.length} ${task.scope.length === 1 ? "file" : "files"}`);
  facts.push(...spentOf(task));
  const row: WorkflowInspectorTask = { id: task.id, title: task.title, said: task.state, facts };
  if (task.touched_after_done) {
    row.flag = "A later task edited a file this one had finished. It stays done.";
  }
  return row;
}

/** The Drone on a task, or the Job's own where the task names none. */
function dronesOf(whole: JobWhole, tasks: GroupView["tasks"]): { id: string; label: string }[] {
  const named = tasks
    .filter((task) => task.drone_id !== undefined)
    .map((task) => ({ id: task.drone_id!, label: `Drone on ${task.id}` }));
  if (named.length > 0) return [...new Map(named.map((one) => [one.id, one])).values()];
  const drone = whole.job.assigned_drone;
  return drone === undefined ? [] : [{ id: drone, label: "This Job's Drone" }];
}

export type WorkflowInspectorReading = {
  whole: JobWhole;
  groups: readonly GroupView[];
  /** The node id a person has open — `step:…` or `group:…`. */
  selected: string | null;
  /** The step that works the groups, as `workflow-canvas.ts` computed it. */
  groupsUnder?: string;
  /** Opens the Plan destination, for the plan card. */
  onOpenPlan?: () => void;
  /** What a running Check measures to. The caller's clock. */
  now?: number;
  /** Every Drone the Job has had, for the track a running step draws. Absent reads none. */
  drones?: readonly DroneView[];
  /** Opens a Check's log panel. Absent leaves the Check rows plain. */
  onOpenCheckLog?: (log: JobCheckLog) => void;
};

/**
 * The step or group a person has open, read whole. `undefined` where the
 * selection names nothing this Job has — a Job switched under a held id.
 */
export function workflowReadingOf({
  whole,
  groups,
  selected,
  groupsUnder,
  onOpenPlan,
  now = Date.now(),
  drones = [],
  onOpenCheckLog,
}: WorkflowInspectorReading): WorkflowReading | undefined {
  if (selected === null) return undefined;

  const group = groups.find((one) => groupNodeId(one.id) === selected);
  if (group !== undefined) {
    const step = ordered(whole).find((one) => one.step_id === groupsUnder);
    return {
      name: `Group ${group.ordinal}`,
      kind: "group",
      doing: doingOfGroup(group),
      tasks: group.tasks.map(taskOf),
      checks: step === undefined ? [] : checksOf(step, now, group.checks_selected),
      checksAbsent: "No Check runs at this group's end.",
      tests: [],
      drones: dronesOf(whole, group.tasks),
    };
  }

  const step = ordered(whole).find((one) => stepNodeId(one.step_id) === selected);
  if (step === undefined) return undefined;
  // **The step that wrote the plan and the step that works it both say so**,
  // and link to it. Neither lists the plan's tasks: they are the Plan
  // destination's, one press away.
  const wrote = step.step_id === stepTheGroupsWereMadeAt(whole);
  const works = step.step_id === groupsUnder;
  const frozen = frozenBeneath(whole.job.status, step.state);
  const activity = frozen?.activity ?? activityOf(step.state);
  const checks = checksOf(step, now, undefined, onOpenCheckLog);
  // The card's own track, so the card and its panel say it together.
  const track = trackOf(whole, step, activity, drones);
  return {
    name: step.label,
    kind: "step",
    eyebrow: `Step ${placeOf(step)}`,
    state: { activity, said: frozen?.word ?? stateOf(step) },
    ...(track === undefined ? {} : { track }),
    ...(wrote || works
      ? {
          plan: {
            groups: groups.map((one) => ({
              id: one.id,
              name: `Group ${one.ordinal}`,
              tasks: `${one.tasks.length} ${one.tasks.length === 1 ? "task" : "tasks"}`,
            })),
            ...(onOpenPlan === undefined ? {} : { onOpen: onOpenPlan }),
          },
        }
      : {}),
    checks,
    checksAbsent: "No Check runs at this step.",
    tests: [],
    // Only where Checks run: a step that makes no diff owes no test, and a
    // note about coverage there would be about nothing it does.
    ...(checks.length === 0
      ? {}
      : { testsAbsent: { says: NO_CASES.says, issue: { href: issueLink(NO_CASES.issue), label: `See #${NO_CASES.issue}.` } } }),
    drones: dronesOf(whole, wrote || works ? groups.flatMap((one) => one.tasks) : []),
  };
}

/** What the group is doing now. Its own word, which the step machine has none of. */
function doingOfGroup(group: GroupView): string {
  const running = group.tasks.filter((task) => task.state === "working").length;
  const beside =
    group.concurrent && group.tasks.length > 1
      ? `${group.tasks.length} tasks at the same time`
      : `${group.tasks.length} ${group.tasks.length === 1 ? "task" : "tasks"}, one after another`;
  const now = running === 0 ? "" : ` ${running} still running.`;
  return `${beside}. This group is ${group.state}.${now}`;
}
