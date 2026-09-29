// The Record's words: where a row happened, who ran it, and what it came to.
//
// **The draft carries facts and this carries English.** `draft/ledger.ts` reads
// a Job's reads into `LedgerRow`s and never picks a sentence; everything a
// person reads on the Record is decided here, once, so the table and the
// inspector cannot disagree about a row.

import type { JobLedgerFilter, JobLedgerRow, LedgerTone, LedgerWho } from "@armada/components";
import type { JobDetail } from "@armada/protocol";

import { absoluteOf, clock } from "./duration";
import { markFor, whatCellOf } from "./record-cells";
import { taskGroupsOf } from "./draft/group";
import {
  countsOf,
  familyOf,
  LEDGER_FAMILIES,
  unfiledIn,
  type LedgerActor,
  type LedgerFamily,
  type LedgerRow,
} from "./draft/ledger";

/** All, and the eight families after it. The filter menu lists them in this order. */
export const RECORD_FILTERS = ["all", ...LEDGER_FAMILIES] as const;

export type RecordFilter = (typeof RECORD_FILTERS)[number];

/** What each filter is called. Sentence case, and the noun the issue names. */
const FILTER_LABEL: Record<RecordFilter, string> = {
  all: "All",
  job: "Job",
  evidence: "Evidence",
  files: "Files",
  checks: "Checks",
  judges: "Judges",
  drones: "Drones",
  tasks: "Tasks",
  tests: "Tests",
};

/**
 * Who ran it, spelled.
 *
 * **A Judge and a Check each get their own entry** (#1530). Both arrive on the
 * wire as Fleet acting, so the derivation is what tells them apart and this is
 * only what they are called.
 */
const WHO_SAYS: Record<LedgerActor, string> = {
  person: "You",
  contributor: "Another contributor",
  fleet: "Fleet",
  drone: "Drone",
  judge: "Judge",
  check: "Check",
};

// `person` is `you` on screen, because the Record is read by the person whose
// machine it is. `contributor` is the other one, and nothing derives it yet.
function whoOf(actor: LedgerActor): LedgerWho {
  return actor === "person" ? "you" : actor;
}

/**
 * The nine filters, with how many rows each holds.
 *
 * **All is the total and the eight are families of it, which is not the same as
 * All being their sum.** `kind` is opaque, so a kind this Bridge has never heard
 * of answers to none of the eight and they can come to less — `unfiledSays` is
 * what tells a reader by how much, rather than leaving them the subtraction.
 */
export function filtersOf(rows: readonly LedgerRow[]): JobLedgerFilter[] {
  const counts = countsOf(rows);
  return RECORD_FILTERS.map((filter) => ({
    id: filter,
    label: FILTER_LABEL[filter],
    count: filter === "all" ? rows.length : counts[filter],
  }));
}

/** One row of a family, named — the open row's eyebrow. */
export const FAMILY_SAYS: Record<LedgerFamily, string> = {
  job: "The Job",
  evidence: "Evidence",
  files: "File",
  checks: "Check",
  judges: "Judge",
  drones: "Drone",
  tasks: "Task",
  tests: "Test",
};

/** The rows one step holds, or every row where no step is chosen. */
export function underStep(rows: readonly LedgerRow[], step: string | null): readonly LedgerRow[] {
  return step === null ? rows : rows.filter((row) => row.coord?.step === step);
}

/** The rows one filter holds, in the order they were composed. */
export function underFilter(rows: readonly LedgerRow[], filter: RecordFilter): LedgerRow[] {
  return filter === "all" ? [...rows] : rows.filter((row) => familyOf(row.kind) === filter);
}

/** One row, as the table draws it. */
export function ledgerRowsFor(
  rows: readonly LedgerRow[],
  detail: JobDetail | null,
): JobLedgerRow[] {
  const steps = new Map((detail?.steps ?? []).map((step) => [step.step_id, step.label]));
  // The ordinal, and how many tasks the group holds — a group of one is not
  // worth a segment beside the task it holds, and today's wire derives one
  // group per task, so naming both would put `group 4 · T4` on every row.
  const groups = new Map(
    (detail === null ? [] : taskGroupsOf(detail)).map((group) => [
      group.id,
      { ordinal: group.ordinal, tasks: group.tasks.length },
    ]),
  );
  return rows.map((row) => {
    const drawn: JobLedgerRow = {
      id: String(row.cursor),
      when: clock(row.at),
      where: whereOf(row, steps, groups),
      who: whoOf(row.actor),
      whoSays: WHO_SAYS[row.actor],
      what: whatCellOf(row),
    };
    const mark = markFor(row.kind);
    if (mark !== undefined) drawn.mark = mark;
    const exact = absoluteOf(row.at);
    if (exact !== null) drawn.whenExact = exact;
    if (row.outcome !== "") drawn.outcome = row.outcome;
    const tone = toneOf(row);
    if (tone !== undefined) drawn.tone = tone;
    return drawn;
  });
}

/**
 * Where a row happened, in words.
 *
 * **A case run with no step reads "after the Job"** (#1530) — a person running
 * a case at review, or a contributor running one on a merged branch. Any other
 * row with no coordinate is the Job's own machine moving, which is a fact about
 * the Job and not a gap.
 */
export function whereOf(
  row: LedgerRow,
  steps: ReadonlyMap<string, string>,
  groups: ReadonlyMap<string, { ordinal: number; tasks: number }>,
): string {
  if (row.coord === null) {
    return familyOf(row.kind) === "tests" ? "After the Job" : "The Job itself";
  }
  const { step, group, task } = row.coord;
  const parts = [steps.get(step) ?? step];
  const held = group === undefined ? undefined : groups.get(group);
  if (held !== undefined && (task === undefined || held.tasks > 1)) {
    parts.push(`group ${held.ordinal}`);
  }
  if (task !== undefined) parts.push(task);
  return parts.join(" · ");
}

/**
 * What a row came to, as a hue.
 *
 * **Read off the outcome's own words and never off the actor.** A Check's row
 * is green or red by what the Check did; a Judge answering `met` is the same
 * green, and a Drone arriving has no outcome and takes no hue at all.
 *
 * **Drift takes no hue, and it drew red until 28 September 2026.** An outcome
 * starting `outside the plan` was in the failing arm below, so a file nobody
 * declared filled its row with `--status-completed-failed` — while `grounds.ts`
 * called the same fact `quiet` and `events.ts` calls `outside_plan` *a mark, not
 * a judgement*. The owner read the red and asked whether it hurt the Job. It
 * does not: the Judge weighs drift and no gate fails on it.
 *
 * **A File row takes no hue at all, before any word is read.** Its outcome can
 * lead with a file's name — `metrics.ts was out of scope` — and a name is not
 * an outcome, however it starts.
 */
export function toneOf(row: LedgerRow): LedgerTone | undefined {
  if (familyOf(row.kind) === "files") return undefined;
  const said = row.outcome.toLowerCase();
  if (said.startsWith("failed") || said.startsWith("not met")) {
    return "failed";
  }
  if (said.startsWith("passed") || said.startsWith("met") || said.startsWith("advanced")) {
    return "passed";
  }
  if (row.kind === "task_working" || row.kind === "drone_spawned") return "running";
  if (row.kind === "flagged" || row.kind === "touched_after_done") return "waiting";
  return undefined;
}

/**
 * What All holds that no filter does, in one line — or nothing, where every
 * row answers to a filter.
 *
 * **The Job filter is what emptied this in the ordinary case.** It said the
 * Job's own moves answered to nothing, which is where the owner stood when he
 * asked for that filter (28 September 2026). What is left is the fact the
 * filters cannot ever cover: `kind` is opaque, so a kind this Bridge has never
 * heard of is a row under All and nowhere else.
 */
export function unfiledSays(rows: readonly LedgerRow[]): string | undefined {
  const count = unfiledIn(rows).length;
  if (count === 0) return undefined;
  return count === 1
    ? "One more row is under All alone: a kind no filter names."
    : `${count} more rows are under All alone: kinds no filter names.`;
}

export type { LedgerFamily };
