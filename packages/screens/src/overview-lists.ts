// What Overview's lists draw, from the Jobs in scope. #920. `OverviewLists.tsx` draws them; this
// decides them, so the arithmetic is tested as arithmetic.
//
// **Nothing here is a second rule.** `sectionsOf` in `board.ts` already says which section a Job
// is drawn under, and `foldLineages` in `lineage.ts` already says a redispatch chain is one row.
// This only scopes the board to the pick and applies both in the order the Board itself applied
// them — fold, then that default sort, then section.
//
// Done was left off while the Job Board still drew it. That surface is gone, and every completed
// or cleared Job would have gone with it, so every section `sectionsOf` returns is drawn here.
//
// **A Job the row shape cannot draw is not a row in any section.** `readingOf` in `reading.ts` is
// the same test `Jobs.tsx` runs before a badge is drawn; a Job that fails it is named instead,
// which is what `undrawable` is for. Named rather than silently dropped — the same choice the
// Board makes for the same case.

import type { JobSummary, RepositorySummary } from "@armada/protocol";
import { DEFAULT_SORT, ofPicked, sectionsOf, sorted } from "./board";
import type { BoardSection } from "./board";
import { foldLineages } from "./lineage";
import type { Dispatch } from "./lineage";
import { readingOf } from "./reading";

export type OverviewSection = { id: BoardSection; label: string; jobs: JobSummary[] };

/** A row in a section, and how far under the Job it waits on it sits. */
export type NestedJob = {
  job: JobSummary;
  /** 0 for a row that waits on nothing in its section. */
  depth: number;
  /** Waits on more than the one it is drawn under, or on one drawn elsewhere. */
  alsoWaits: boolean;
};

/**
 * A section's Jobs with each dependent beneath the Job it waits on.
 *
 * **A Job with several parents is drawn once, under the first of them that
 * this section holds**, and marked. A parent in another section leaves the
 * dependent a root here: nesting never moves a Job out of the section its
 * state puts it in. A cycle stops where it closes, leaving the rest roots.
 */
export function nestedOf(jobs: readonly JobSummary[]): NestedJob[] {
  const held = new Set(jobs.map((job) => job.id));
  const parentOf = (job: JobSummary): string | undefined =>
    (job.waits_on ?? []).find((id) => held.has(id) && id !== job.id);
  const children = new Map<string, JobSummary[]>();
  for (const job of jobs) {
    const parent = parentOf(job);
    if (parent !== undefined) children.set(parent, [...(children.get(parent) ?? []), job]);
  }
  const out: NestedJob[] = [];
  const drawn = new Set<string>();
  const place = (job: JobSummary, depth: number) => {
    if (drawn.has(job.id)) return;
    drawn.add(job.id);
    out.push({ job, depth, alsoWaits: (job.waits_on ?? []).length > (depth === 0 ? 0 : 1) });
    for (const child of children.get(job.id) ?? []) place(child, depth + 1);
  };
  for (const job of jobs) if (parentOf(job) === undefined) place(job, 0);
  // Only a cycle leaves any undrawn.
  for (const job of jobs) place(job, 0);
  return out;
}

export type OverviewListsRead = {
  /**
   * Every section `sectionsOf` carries, and one with nothing in it left off.
   *
   * **Recently ended is still not Done.** `sectionsOf` carves the terminal
   * statuses that owe something out of Done and leaves `completed_success` and
   * `superseded` in it — Overview 28 (#1092)'s split, unchanged by Done
   * arriving beside it.
   */
  sections: OverviewSection[];
  /** Which dispatch of its lineage each folded-in Job is, keyed by id — `headlineOf`'s second argument. */
  dispatch: ReadonlyMap<string, Dispatch>;
  /** Jobs in scope the row shape cannot draw, oldest first — named beneath the lists, drawn by neither. */
  undrawable: JobSummary[];
};

/**
 * Overview's lists, scoped to the pick and ready to draw as the Board's own rows.
 *
 * `picked` is `null` for All repositories, and `ofPicked`'s own term otherwise — the same term
 * `OverviewSummary` resolves before reading this, so the strip's counts and the panels below it
 * never drift apart.
 */
export function overviewListsOf(jobs: readonly JobSummary[], picked: RepositorySummary | null): OverviewListsRead {
  const board = foldLineages(ofPicked(jobs, picked));
  const shown = sorted(board.shown, DEFAULT_SORT);
  const drawn = shown.filter((job) => readingOf(job).as === "badge");
  const undrawable = shown.filter((job) => readingOf(job).as !== "badge");
  const sections = sectionsOf(drawn);
  return { sections, dispatch: board.dispatch, undrawable };
}
