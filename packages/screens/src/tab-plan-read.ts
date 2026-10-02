// The Plan destination's own read of a Job — pure, so the arithmetic and every
// sentence on the board are tested without a browser. `#1535`.
//
// **Every word the board draws is written here.** `PlanBoard` composes no
// prose, for `StepBar`'s reason: a count in words is copy, and copy has one
// owner. So does the rule that decides it.

import type { PlanBoardTest } from "@armada/components";
import type { PlanTaskSheetProps, PlanTaskTest, TaskMarkState } from "@armada/components";
import { GROUP_STATE } from "@armada/components";
import type { JobDetail, StepDetail } from "@armada/protocol";

import { caseViewsOf, scopeRevisionsOf, type CaseView, type ScopeRevisionView } from "./draft/cases";
import { criterionViewsOf, type CriterionView } from "./draft/criterion";
import { taskGroupsOf, type GroupState, type GroupView } from "./draft/group";
import type { JobDraft } from "./draft/held";
import { planRevisionsOf, type PlanRevisionView } from "./draft/revision";
import type { TaskView } from "./draft/task";
import { money } from "./facts";

/**
 * The plan's groups. **The draft where a moment carries one, and today's wire
 * derived otherwise** — `taskGroupsOf` draws one task per group, so a board
 * built here renders against the real Fleet thinner rather than empty.
 */
export function groupsOf(whole: JobDetail | null, draft?: JobDraft): GroupView[] {
  if (draft?.groups !== undefined) return [...draft.groups];
  return whole === null ? [] : taskGroupsOf(whole);
}

/** The cases, on the same terms: the draft's, or the specs the Job's Drones named. */
export function casesOf(whole: JobDetail | null, draft?: JobDraft): CaseView[] {
  if (draft?.cases !== undefined) return [...draft.cases];
  return whole === null ? [] : caseViewsOf(whole);
}

/** What the Job is held to, on the same terms. */
export function criteriaOf(whole: JobDetail | null, draft?: JobDraft): CriterionView[] {
  if (draft?.criteria !== undefined) return [...draft.criteria];
  return whole === null ? [] : criterionViewsOf(whole);
}

/** The changes asked of the plan's Drone, on the same terms. */
export function scopeRevisionsCarriedBy(
  whole: JobDetail | null,
  draft?: JobDraft,
): ScopeRevisionView[] {
  if (draft?.scope_revisions !== undefined) return [...draft.scope_revisions];
  return whole === null ? [] : scopeRevisionsOf(whole);
}

/** Each change asked for, paired with the Judge's answer to it. */
export function revisionsOf(
  whole: JobDetail | null,
  draft: JobDraft | undefined,
  step: StepDetail | undefined,
): PlanRevisionView[] {
  const groups = groupsOf(whole, draft);
  return planRevisionsOf(
    step,
    scopeRevisionsCarriedBy(whole, draft),
    casesOf(whole, draft),
    groups,
    criteriaOf(whole, draft),
  );
}

/** Every task of every group, in the order the groups run. */
export function tasksOf(groups: readonly GroupView[]): TaskView[] {
  return groups.flatMap((group) => group.tasks);
}

/**
 * Where a group is, in words: `GROUP_STATE`'s verb, from `enum-verbs.toml`, so
 * the list, the panel, the graph and Overview say one word for one state.
 */
export function groupSaid(state: GroupState): string {
  return GROUP_STATE[state]?.verb ?? state;
}

/**
 * What a case reads as. **A case with no spec is `not covered`**, never green
 * and never a pass by having nothing to run — `#1530`, 21 Sep.
 */
export function caseReads(one: CaseView): PlanBoardTest["reads"] {
  if (one.state === "dropped") return "dropped";
  return one.has_spec ? "owed" : "not covered";
}

/** What dropped a case, as one line. Nothing where it is still owed. */
export function droppedSaid(one: CaseView): string | undefined {
  const by = one.dropped_by;
  if (by === undefined) return undefined;
  if (by.dropped === "scope_revision") return "dropped by a scope revision";
  const where = by.coord.task ?? by.coord.group ?? by.coord.step;
  return `dropped on a retry of ${where}`;
}

/** One case as the board and the inspector both draw it. */
function testOf(one: CaseView): PlanBoardTest {
  const dropped = droppedSaid(one);
  return {
    id: one.id,
    spec: one.spec,
    reads: caseReads(one),
    ...(dropped === undefined ? {} : { droppedSays: dropped }),
  };
}

// The Job's Drone cap was drawn on a concurrent group's chip until 28 Sep,
// joined to the shape behind a middle dot. The owner cut it: `2 tasks run at
// the same time · this Job runs 2 Drones at once` is two facts in one
// sentence, and the second is the Job's rather than the group's. Overview
// carries it, frozen at the gate, as `Drones at once`.

/** `34 turns`, for the row's own column. Nothing before its agent started. */
export function turnsSaid(task: TaskView): string | undefined {
  return task.turns === undefined ? undefined : `${task.turns} turns`;
}

/** `~$2.40`, for the row's own column. Nothing until its agent stopped. */
export function costSaid(task: TaskView): string | undefined {
  return task.cost_micros === undefined ? undefined : money(task.cost_micros);
}

/** How a task is run, in words a person reads rather than the wire's enum. */
export function runBySaid(task: TaskView): string {
  switch (task.treatment) {
    case "own_drone":
      return "its own agent";
    case "job":
      return "a Job of its own";
    default:
      return "the step's Drone";
  }
}

/** `beside T5`. Nothing where the task runs alone. */
export function besideSaid(task: TaskView): string | undefined {
  return task.concurrent_with.length === 0
    ? undefined
    : `beside ${task.concurrent_with.join(", ")}`;
}

/**
 * Which later task reached into a finished one's files, by task id.
 *
 * **Derived rather than served.** `touched_after_done` says a later task
 * edited this one's file and never which; the plan's own order and scopes say
 * it exactly, and a flag naming nobody is a flag a person cannot act on.
 */
export function touchedByOf(groups: readonly GroupView[]): Map<string, string> {
  const order = tasksOf(groups);
  const found = new Map<string, string>();
  order.forEach((task, at) => {
    if (!task.touched_after_done) return;
    const claimed = new Set(task.scope);
    const later = order
      .slice(at + 1)
      .find((candidate) => candidate.scope.some((path) => claimed.has(path)));
    if (later !== undefined) found.set(task.id, later.id);
  });
  return found;
}

/**
 * Where a group writes, as one value: the deepest directory every path it
 * claims sits under, and how many paths that is.
 *
 * **A root rather than the list.** The list said neither where the gate
 * measures the diff nor where the next group collides (owner, 28 Sep 2026);
 * the root says the first exactly, and `overlapsOf` says the second.
 */
export function scopeRootOf(paths: readonly string[]): { root: string } {
  if (paths.length === 0) return { root: "the whole repository" };
  const [first, ...rest] = paths;
  let common = (first ?? "").split("/").slice(0, -1);
  for (const path of rest) {
    const parts = path.split("/").slice(0, -1);
    let at = 0;
    while (at < common.length && at < parts.length && common[at] === parts[at]) at += 1;
    common = common.slice(0, at);
  }
  const root = common.length === 0 ? "**" : `${common.join("/")}/**`;
  return { root };
}

// The callout's future waits on [case-waits-for-last-group] in docs/concepts/plan.md.
/**
 * Which files each group shares with another, keyed by group id.
 *
 * **On the group's own card**, the owner's call of 28 Sep 2026: a warning
 * that group 4 overlaps group 3 belongs on group 4, not in a band above the
 * plan where it names two groups a reader then has to go and find.
 */
export function overlapsOf(
  groups: readonly GroupView[],
): Map<string, { says: string; paths: string[] }[]> {
  const claimed = new Map<string, Set<string>>();
  for (const group of groups) {
    claimed.set(group.id, new Set(group.tasks.flatMap((task) => task.scope)));
  }
  const found = new Map<string, { says: string; paths: string[] }[]>();
  for (const group of groups) {
    const mine = claimed.get(group.id) ?? new Set<string>();
    const shared: { says: string; paths: string[] }[] = [];
    for (const other of groups) {
      if (other.id === group.id) continue;
      const theirs = claimed.get(other.id) ?? new Set<string>();
      const both = [...mine].filter((path) => theirs.has(path));
      if (both.length === 0) continue;
      shared.push({ says: `Group ${other.ordinal} writes these files too`, paths: both });
    }
    if (shared.length > 0) found.set(group.id, shared);
  }
  return found;
}

/**
 * Proposing a change, on a group or on a task — **one verb for both** (owner,
 * 30 Sep 2026). Prose the Drone reads and may refuse; every other change to a
 * plan is a person's own, made directly through Fleet. No placeholder: an
 * empty field stays empty.
 */
export const PROPOSE_ASK = {
  label: "Propose a change",
  send: "Send to the Drone",
} as const;

/** Removing a group: each of its tasks dropped, with one reason. */
export const REMOVE_GROUP_LABEL = "Remove";

/** The mark a task's row leads with. `TaskMark` prints no word beside it. */
export function markOf(state: TaskView["state"]): TaskMarkState {
  return state;
}

/**
 * Which Checks failed at a group's boundary, by name.
 *
 * Read off the step's own `check_runs`, **the group's own latest run where
 * Fleet records the group** (#1652). A Fleet before 23.4 named none, and there
 * **only a group that is carrying the failure takes it** — the step's runs are
 * one list for every group in it, so attributing them by run alone would paint
 * a passed group with another's red.
 */
export function failedChecksOf(whole: JobDetail | null, group: GroupView): string[] {
  if (whole === null) return [];
  const step = whole.steps.find((one) => one.step_id === whole.job.current_step_id);
  const runs = step?.check_runs ?? [];
  const own = runs.filter((run) => run.group === group.id);
  if (own.length > 0) {
    const latest = Math.max(...own.map((run) => run.group_attempt ?? 0));
    return own
      .filter((run) => (run.group_attempt ?? 0) === latest && run.outcome === "failed")
      .map((run) => run.name);
  }
  const carrying = group.state === "failed" || group.state === "retrying";
  if (!carrying || runs.some((run) => run.group !== undefined)) return [];
  const latest = Math.max(0, ...runs.map((run) => run.attempt));
  return runs
    .filter((run) => run.attempt === latest && run.outcome === "failed")
    .map((run) => run.name);
}

/**
 * How many times a group has been run. **Nothing on its first run** — a count
 * of one would read as a retry that has not happened.
 *
 * **`attempt 2`, the app's one word for a run again** — a step's card on the
 * Workflow canvas says `attempt 2` in the same chip. `second run` read as
 * `second r…` once the strip cut it (the owner, 29 Sep 2026).
 */
export function retrySaid(retries: number): string | undefined {
  if (retries <= 0) return undefined;
  return `attempt ${retries + 1}`;
}

// The group card and the whole board are `plan-board.ts`'s: composing a card
// needs both these sentences and what only a group that has run says, and one
// direction of import is the price of not spelling either twice.

/**
 * Which Drone a correction about this task reaches, and what to call it.
 *
 * **Labelled for what it actually reaches.** A task with an agent of its own is
 * addressed by task; the Job's one Drone is the fallback and says so, because
 * Fleet runs one per Job and calling it `Drone on T5` would be a claim the wire
 * does not make. `undefined` where there is no Drone at all.
 */
export function droneOfTask(
  whole: JobDetail | null,
  task: TaskView,
): { id: string; label: string } | undefined {
  if (task.drone_id !== undefined) return { id: task.drone_id, label: `Drone on ${task.id}` };
  return jobDroneOf(whole);
}

/** The Job's one Drone, where Fleet names one — what a row naming no task reaches. */
export function jobDroneOf(whole: JobDetail | null): { id: string; label: string } | undefined {
  const job = whole?.job.assigned_drone;
  return job === undefined ? undefined : { id: job, label: "This Job's Drone" };
}

/**
 * The inspector's own reading of one task. `undefined` where the plan holds no
 * task by that id, which is a sheet that should not be open.
 *
 * **`beside` stays ids here.** The sheet draws each as the graph's card, and
 * the card's press is the caller's — `tab-plan.tsx` builds them.
 */
export function taskSheetOf(
  taskId: string,
  groups: readonly GroupView[],
  cases: readonly CaseView[],
): (Omit<PlanTaskSheetProps, "open" | "beside"> & { beside: readonly string[] }) | undefined {
  const task = tasksOf(groups).find((one) => one.id === taskId);
  if (task === undefined) return undefined;
  const owed: PlanTaskTest[] = cases
    .filter((one) => one.tasks.includes(task.id) || task.cases.includes(one.id))
    .map(testOf);
  // The flag the plan list carries as `touched later · T7`, back in the panel
  // (owner, 30 Sep 2026) after it left with the old brief field.
  const later = touchedByOf(groups).get(task.id);
  return {
    id: task.id,
    title: task.title,
    state: markOf(task.state),
    scope: task.scope,
    // Drawn while [case-waits-for-last-group] is open — whether the overlap is worth saying at all.
    ...(later === undefined ? {} : { overlap: `${later} edited a file this task had already finished` }),
    tier: task.tier,
    model: task.model,
    beside: task.concurrent_with,
    tests: owed,
    ...(task.note === undefined ? {} : { note: task.note }),
    ...(task.expects === undefined ? {} : { expects: task.expects }),
    ...(task.shown === undefined ? {} : { shown: task.shown }),
    ...(task.reason === undefined ? {} : { reason: task.reason }),
    ...(task.failed_reason === undefined ? {} : { failedReason: task.failed_reason }),
  };
}
