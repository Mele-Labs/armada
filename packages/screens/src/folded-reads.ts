// The reads a Job detail folds into one value. A file of its own because the
// fixtures' shared core types a Job with it and must not import the Jobs surface.

import type { Diff, Evidence, Footprint, Handed, Remarks } from "@armada/protocol";

/**
 * The reads the panel's own chapters draw from. `JobDetail.tsx` re-exports it,
 * which is where every caller already imports it from.
 */
export type FoldedReads = {
  footprint: Footprint;
  /**
   * The moment the Job's Drone handed in, before the gate started. Pushed, not
   * fetched — `footprint`'s terms — and it says only that a submission landed.
   * What was in it is `evidence`, which is asked for. `#813`.
   */
  handed: Handed;
  evidence: Evidence;
  diff: Diff;
  /**
   * What people wrote on the Job's pull request, where the decision block asked
   * for it. **The one read in this set that costs a forge**, so nothing takes
   * it on a timer and no event refreshes it.
   */
  remarks: Remarks;
};
