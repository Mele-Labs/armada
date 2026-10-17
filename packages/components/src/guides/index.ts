// Every guide, in catalogue order. **This list is the only transcription.**
//
// One guide per file, so a change to one is a diff of one; one array here, so
// the catalogue, the marks and the first-contact bookkeeping all read the same
// set. Nothing globs the directory: the order is a reading order rather than
// an alphabet, and a glob would decide it by filename.
//
// **A number is the guide's own for good.** A guide added takes the next
// number above the highest in use and is filed wherever its group wants it, so
// the catalogue reads in group order and not in number order. Guides 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11,
// 12, 13, 14, 20 and 21 are retired and gone. `guides.test.ts` holds both rules.

import type { Guide, GuideGroupId } from "./guide";
import { GUIDE_JOB } from "./015-what-is-a-job";
import { GUIDE_BEFORE_A_RUN } from "./016-what-can-i-change-before-a-job-runs";
import { GUIDE_PLAN } from "./017-what-is-a-plan";
import { GUIDE_DRONE } from "./018-what-is-a-drone";
import { GUIDE_WORKFLOW } from "./019-what-is-a-workflow";
import { GUIDE_MERGE_LINE } from "./022-what-is-the-merge-line";

export * from "./guide";
export {
  GUIDE_JOB,
  GUIDE_BEFORE_A_RUN,
  GUIDE_PLAN,
  GUIDE_DRONE,
  GUIDE_WORKFLOW,
  GUIDE_MERGE_LINE,
};

/**
 * The numbers no guide may take. **A retired guide keeps its number.**
 *
 * 2 explained the link a Job's member carries to the one before it, and went with the members list.
 *
 * 11 explained the second edge a group gets on the Workflow canvas. The plan's
 * graph moved to the Plan tab on 25 September 2026, so the guide had nothing
 * left to explain and was retired rather than rewritten.
 *
 * 5 explained the bar under a plan group, which went when the plan board lost its
 * last screen with the Jobs deletion of October 2026.
 *
 * 8 explained the run tree's step bar, which went with the Overview reframe of
 * 29 September 2026. 20 explained drift, and no screen draws `run.drift`.
 *
 * 1, 3, 4, 6, 7, 9, 10, 12, 13, 14 and 21 explained pieces of the Job screens, the
 * plan board, the run sheet and the Pulse, which went with the Jobs deletion of
 * October 2026 and left no screen that draws them.
 */
export const RETIRED_GUIDE_NUMBERS: readonly number[] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 20, 21];

export const GUIDES: readonly Guide[] = [
  GUIDE_JOB,
  GUIDE_BEFORE_A_RUN,
  GUIDE_PLAN,
  GUIDE_DRONE,
  GUIDE_MERGE_LINE,
  GUIDE_WORKFLOW,
];

/** The guides filed under one group, in catalogue order. */
export function guidesIn(group: GuideGroupId): readonly Guide[] {
  return GUIDES.filter((guide) => guide.group === group);
}

/** The guide explaining a piece, or nothing where no guide claims it. */
export function guideForPiece(piece: string): Guide | undefined {
  return GUIDES.find((guide) => guide.piece === piece);
}
