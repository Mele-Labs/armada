// A group of tasks, and the boundary its Checks run at. Draft, over
// `WorkPlan.groups` in `crates/ipc/src/work_plan.rs`.
//
// Source of truth since 23.4: Fleet's `PlanGroup` — its id, its tasks, its
// state and each run — with the `StepDetail` of the step that works the tasks
// for its Checks. A Fleet before that served no group, and the tasks' own
// states and that step stand in for one.
//
// **Checks run at each group's end** (#1530, 21 Sep), which is why the group
// and not the task is what carries a verdict, a commit and a retry count.

import type { DeclaredCheck, JobDetail, PlanGroup, StepDetail } from "@armada/protocol";

import { derivedGroupId, stepAttemptOf } from "./coord";
import type { LedgerRow } from "./ledger";
import { taskViewsOf, type TaskState, type TaskView } from "./task";

/**
 * Where a group is. Eight values, and none of them is a Job status: a group is
 * inside one step of one Job, and the Job's own machine does not move when a
 * group does.
 */
export type GroupState =
  | "pending"
  | "running"
  | "joining"
  | "checking"
  | "passed"
  | "failed"
  | "retrying"
  | "landed";

/** One group of a step's work. */
export type GroupView = {
  /** Stable for the life of the plan. Never the ordinal — see `RunCoord`. */
  id: string;
  /** Its position in the step, counted from one. Moves when a group does. */
  ordinal: number;
  tasks: TaskView[];
  /** Every path the group's tasks claim, de-duplicated, in first-seen order. */
  scope: string[];
  /** Whether its tasks run at the same time. */
  concurrent: boolean;
  state: GroupState;
  /** The Checks selected to run at this group's boundary, by name. */
  checks_selected: string[];
  /**
   * The cases that run at this boundary, by id. **A case runs at the boundary
   * of the last group holding a task it covers**, and again at handoff
   * (#1530, 21 Sep).
   */
  cases_at_boundary: string[];
  /**
   * When the group started and ended, as Fleet stamps them since 23.4. Absent
   * from an older Fleet, and on a group that has not run: a surface says it is
   * not timed rather than drawing a dash or a zero.
   */
  started_at?: string;
  ended_at?: string;
  /** What the boundary came to. Absent until the Checks have run. */
  verdict?: string;
  /** How many times this group has been run again after a failure. */
  retry_count: number;
  /** The commit the group left, where it left one. */
  commit?: string;
};

/**
 * The plan's groups, **as Fleet serves them since 23.4**: its ids, its state
 * and its runs. A Fleet before that served none, so **one task per group, in
 * plan order** — a Job with six tasks draws six groups of one rather than
 * nothing at all.
 *
 * **A group's Checks are the step that works the tasks'** — never the step the
 * Job is on. A Job on its plan step is on the step that *writes* the tasks:
 * Job 2 (owner, 1 Oct 2026) drew four groups running and `plan_recorded` at
 * every boundary while nothing had started.
 */
export function taskGroupsOf(detail: JobDetail): GroupView[] {
  const served = detail.work_plan?.groups;
  if (served !== undefined && served.length > 0) return servedGroupsOf(detail, served);
  const worksAt = stepWorkingTheTasks(detail);
  const attempts = stepAttemptOf(worksAt);
  const commit = detail.delivery?.commit;

  return taskViewsOf(detail).map((task, index) => {
    const group: GroupView = {
      id: derivedGroupId(task.id),
      ordinal: index + 1,
      tasks: [task],
      scope: [...task.scope],
      concurrent: false,
      state: groupStateOf(task.state, worksAt),
      checks_selected: checksReaching(worksAt, task.scope),
      cases_at_boundary: [],
      // A step's `attempts` counts runs of the step, and with one group per
      // step a second run of the step is a second run of every group in it.
      retry_count: attempts - 1,
    };
    if (worksAt?.last_verdict !== undefined) group.verdict = worksAt.last_verdict.named;
    if (commit !== undefined) group.commit = commit;
    return group;
  });
}

/**
 * Fleet's groups, each holding its own tasks. **A group a move emptied draws
 * nothing**, so the ordinals count the groups a person can see.
 */
function servedGroupsOf(detail: JobDetail, served: readonly PlanGroup[]): GroupView[] {
  const worksAt = stepWorkingTheTasks(detail);
  const tasks = taskViewsOf(detail);
  return served
    .map((group) => ({ group, own: tasks.filter((task) => group.tasks.includes(task.id)) }))
    .filter(({ own }) => own.length > 0)
    .map(({ group, own }, index) => {
      const scope = [...new Set(own.flatMap((task) => task.scope))];
      const runs = group.attempts ?? [];
      const view: GroupView = {
        id: group.id,
        ordinal: index + 1,
        tasks: own,
        scope,
        concurrent: false,
        state: groupStateServed(group.state),
        checks_selected: checksReaching(worksAt, scope),
        cases_at_boundary: [],
        retry_count: Math.max(0, runs.length - 1),
      };
      const verdict = runs.at(-1)?.verdict?.named;
      const commit = [...runs].reverse().find((run) => run.commit !== undefined)?.commit;
      if (group.started_at !== undefined) view.started_at = group.started_at;
      if (group.ended_at !== undefined) view.ended_at = group.ended_at;
      if (verdict !== undefined) view.verdict = verdict;
      if (commit !== undefined) view.commit = commit;
      return view;
    });
}

/** A served state, as one of the eight; a word this build lacks reads `pending`. */
function groupStateServed(state: string): GroupState {
  const known: readonly GroupState[] = [
    "pending",
    "running",
    "joining",
    "checking",
    "passed",
    "failed",
    "retrying",
    "landed",
  ];
  return known.find((one) => one === state) ?? "pending";
}

/**
 * The step that made the groups: the one the plan was recorded at.
 *
 * Absent where no plan was recorded and where a person wrote it — a plan no
 * step produced has no node to come off, so its groups are left undrawn rather
 * than hung somewhere they were not made.
 */
export function stepTheGroupsWereMadeAt(whole: JobDetail): string | undefined {
  const at = whole.work_plan?.recorded_by;
  if (at === undefined || at.by !== "step") return undefined;
  return whole.steps.some((step) => step.step_id === at.step_id) ? at.step_id : undefined;
}

/**
 * The step that works the groups: the one after the step the plan was recorded
 * at, since a plan is written at one step and worked at the next.
 *
 * Absent where the recording step is the last. The Workflow canvas reads it for
 * what a step's card counts, the Plan board for where a boundary's Check runs
 * are, and `taskGroupsOf` for which Checks a boundary holds.
 */
export function stepThatWorksTheGroups(whole: JobDetail): string | undefined {
  const made = stepTheGroupsWereMadeAt(whole);
  if (made === undefined) return undefined;
  const steps = [...whole.steps].sort((a, b) => a.ordinal - b.ordinal);
  return steps[steps.findIndex((step) => step.step_id === made) + 1]?.step_id;
}

/**
 * `stepThatWorksTheGroups`, or the step the Job is on where no step it holds
 * recorded the plan: a plan a person wrote is worked where the Job is.
 */
function stepWorkingTheTasks(detail: JobDetail): StepDetail | undefined {
  const named =
    stepTheGroupsWereMadeAt(detail) === undefined
      ? detail.job.current_step_id
      : stepThatWorksTheGroups(detail);
  return detail.steps.find((step) => step.step_id === named);
}

/**
 * The working step's Manifest Checks that reach a task's paths, by name.
 *
 * **Only `manifest_check`**: `diff_nonempty` is the step's own guard and
 * `plan_recorded` the plan step's, and neither is a task's. **A Check with no
 * `when` reaches every task**, as it reaches every change at Fleet's gate.
 */
function checksReaching(step: StepDetail | undefined, scope: readonly string[]): string[] {
  const reaches = (check: DeclaredCheck): boolean =>
    check.when === undefined ||
    check.when.some((pattern) => scope.some((path) => covers(pattern, path)));
  return (step?.checks ?? []).flatMap((check) =>
    check.kind === "manifest_check" && check.name !== undefined && reaches(check)
      ? [check.name]
      : [],
  );
}

/**
 * Whether one `when` pattern covers one path, in Armada's dialect and no other:
 * literal text, `*` inside one segment, `**` for zero or more whole segments
 * (`docs/contracts/configuration.md`). Fleet's matcher is `walk` in
 * `crates/core-model/src/job/covers.rs`; this is it line for line, because the
 * dialect is that small and a glob library reads a wider one.
 */
function covers(pattern: string, path: string): boolean {
  return walk(pattern.split("/"), path.replace(/^\/+/, "").split("/"));
}

function walk(segments: readonly string[], parts: readonly string[]): boolean {
  const [segment, ...rest] = segments;
  if (segment === undefined) return parts.length === 0;
  if (segment === "**") {
    for (let taken = 0; taken <= parts.length; taken += 1) {
      if (walk(rest, parts.slice(taken))) return true;
    }
    return false;
  }
  const [part, ...tail] = parts;
  return part !== undefined && oneSegment(segment, part) && walk(rest, tail);
}

function oneSegment(named: string, part: string): boolean {
  const star = named.indexOf("*");
  if (star === -1) return named === part;
  const before = named.slice(0, star);
  if (!part.startsWith(before)) return false;
  const rest = part.slice(before.length);
  for (let taken = 0; taken <= rest.length; taken += 1) {
    if (oneSegment(named.slice(star + 1), rest.slice(taken))) return true;
  }
  return false;
}

/**
 * The groups, timed by what the Record says happened inside each one.
 *
 * **The first and last row recorded at a group, and nothing invented**: a task
 * carries no instants either, so this is the only source a board has. A group
 * with no rows of its own keeps both fields absent and reads as not timed.
 */
export function groupsTimedBy(
  groups: readonly GroupView[],
  rows: readonly LedgerRow[],
): GroupView[] {
  return groups.map((group) => {
    const at = rows
      .filter((row) => row.coord?.group === group.id)
      .map((row) => row.at)
      .sort();
    const first = at[0];
    const last = at[at.length - 1];
    if (first === undefined || last === undefined || first === last) return group;
    return { ...group, started_at: first, ended_at: last };
  });
}

// Where a group is: **its task's state, and the working step's only where that
// step has settled something about every task in it** — its Checks underway, or
// its run passed, stopped or retrying. A step merely running or waiting says
// nothing about which task is under way; the task does. `domain/step-states.toml`
// declares six step states — `advanced`, `awaiting_human`, `not_started`,
// `retrying`, `running`, `stopped` — and none of them is `joining`, `checking`
// or `landed`; `checking` is read off `StepDetail.checking` rather than out of
// that set. A dropped task's group never starts, so it reads as not started.
function groupStateOf(task: TaskState, worksAt: StepDetail | undefined): GroupState {
  if (task === "done") return "passed";
  if (task === "failed") return "failed";
  if (task === "dropped") return "pending";
  if (worksAt?.checking !== undefined) return "checking";
  switch (worksAt?.state) {
    case "advanced":
      return "passed";
    case "stopped":
      return "failed";
    case "retrying":
      return "retrying";
    default:
      // A handed-in task's group is live until its Checks answer.
      return task === "working" || task === "handed_in" ? "running" : "pending";
  }
}
