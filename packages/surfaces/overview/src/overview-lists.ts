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
import { DEFAULT_SORT, ofPicked, sectionsOf, sorted } from "@armada/screens/src/board";
import type { BoardSection } from "@armada/screens/src/board";
import { foldLineages } from "./lineage";
import type { Dispatch } from "./lineage";
import { readingOf } from "@armada/screens/src/reading";

export type OverviewSection = { id: BoardSection; label: string; jobs: JobSummary[] };

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
