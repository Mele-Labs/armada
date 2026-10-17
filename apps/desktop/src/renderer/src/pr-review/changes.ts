// The changes the reader draws, and which of them a thread or a draft of his is on. **A thread is named by its place in a
// file** (`path`, `line`, `side`), as the forge names it, so any view of the same file finds it: a change an explanation
// grouped, or the bare file. The change that owns a place is the first that holds that line of that path.

import type { Change, Draft, Finding, PrWork, ReviewPr, Thread } from "./model";
import { indexOfPlace, type Place } from "./patch";

/** Where on a change a place is drawn: the hunk, and the line of it. No line is the change itself. */
export type Spot = { hunk: number; line?: number };

/** A change nobody grouped is a file: it is named by its path, and no explanation placed it. */
export const isFile = (change: Change): boolean => change.at === undefined;

/** The path of the file whose patch a change draws. */
export const fileOf = (change: Change): string => change.at?.path ?? change.id;

/** The place a finding is on: where its range begins. */
export const placeOfFinding = (finding: Pick<Finding, "at">): Place => ({ line: finding.at.start, side: finding.at.side });

const fileChange = (path: string, work: Pick<PrWork, "patches">): Change => {
  const read = work.patches[path];
  return { id: path, title: path, why: "", hunks: read?.state === "read" ? [{ path, lines: read.lines }] : [] };
};

/**
 * The changes of a pull request. Where an explanation grouped them, its themes' changes with their hunks; otherwise **one a
 * file**, whose hunk is the patch once it has been read from the forge, and none before. **A finding no change holds is never
 * dropped**: its file is a change of its own, after the others.
 */
export function changesOf(pr: ReviewPr, work: Pick<PrWork, "patches" | "findings">): Change[] {
  const themed = pr.explanation.themes.flatMap((theme) => theme.changes);
  const base = themed.length > 0 ? themed : pr.page.files.map((file) => fileChange(file.path, work));
  const loose = [...new Set(work.findings.filter((one) => !one.dismissed && ownerOf(base, one.at.path, placeOfFinding(one)) === undefined).map((one) => one.at.path))];
  return [...base, ...loose.map((path) => fileChange(path, work))];
}

/** The paths a change touches. A bare file's change is named by its path, so it answers before its patch is read. */
const pathsOf = (change: Change): string[] => (change.hunks.length > 0 ? change.hunks.map((hunk) => hunk.path) : [fileOf(change)]);

/** The place a thread is on, or none where it is on the file as a whole. */
export const placeOfThread = (thread: Pick<Thread, "line" | "side">): Place | undefined => (thread.line === undefined ? undefined : { line: thread.line, side: thread.side });

/** The place a draft is on, or none where it is on the file as a whole. */
export const placeOfDraft = (draft: Pick<Draft, "line" | "side">): Place | undefined => (draft.line === undefined || draft.side === undefined ? undefined : { line: draft.line, side: draft.side });

/** Where on `change` a place in `path` is drawn. Undefined where no hunk it has holds that line, which draws it under the change's head. */
export function spotOf(change: Change, path: string, place: Place | undefined): Spot | undefined {
  const hunks = change.hunks.flatMap((hunk, at) => (hunk.path === path ? [{ hunk, at }] : []));
  if (place === undefined) return hunks[0] === undefined ? undefined : { hunk: hunks[0].at };
  for (const { hunk, at } of hunks) {
    const line = indexOfPlace(hunk.lines, place);
    if (line >= 0) return { hunk: at, line };
  }
  return undefined;
}

/** The change a place in `path` belongs to: the first that holds the line, else the first that holds the path. */
export function ownerOf(all: readonly Change[], path: string, place: Place | undefined): Change | undefined {
  const holding = all.filter((change) => pathsOf(change).includes(path));
  if (place === undefined) return holding[0];
  return holding.find((change) => spotOf(change, path, place) !== undefined) ?? holding[0];
}
