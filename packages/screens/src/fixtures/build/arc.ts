// The arc: one Feature Job, fifteen moments, from an empty prompt to a merge.
//
// **The Job is #1162**, "Show what's running in the Drones stat", on branch
// `armada/3-show-what-s-running-in-the-drones-stat` — the work the new boards
// were drawn against. Each moment freezes it once, and the moments before the
// approval press freeze the Board it would have joined instead.
//
// **The roster is walked, never listed twice.** `arc.test.tsx` and the mock's
// scenario list both read `ARC_MOMENTS`, so a moment added here is a scenario
// and a claim without a second edit.
//
// **A moment is every board's, so one board never reshapes it.** 29 Sep 2026:
// the Drones board wanted two running Drones and set T6 working in
// `executingSequential`; Plan then drew "2 working" on a moment that says one,
// and only Plan's own test noticed. Give one board more through its own draft
// field, or add a moment.

import type { ArcMoment } from "./arc-base";
import { dispatchSketch, dispatchTyping } from "./arc-dispatch";
import {
  approvedFrozen,
  proposingDispatched,
  proposingFilling,
  proposingReading,
  proposingReview,
  proposingSlow,
} from "./arc-proposing";
import { plannedMoment, planRevisionRefused } from "./arc-planning";
import {
  doneTouched,
  executingConcurrent,
  executingSequential,
  groupFailed,
} from "./arc-executing";
import { landed } from "./arc-landed";

export type { ArcDraft, ArcMoment } from "./arc-base";
export {
  ARC_BRANCH,
  ARC_HANDLE,
  ARC_JOB_ID,
  ARC_NOW,
  ARC_TITLE,
} from "./arc-base";
export { dispatchSketch, dispatchTyping };
export {
  approvedFrozen,
  proposingDispatched,
  proposingFilling,
  proposingReading,
  proposingReview,
  proposingSlow,
};
export { plannedMoment, planRevisionRefused };
export { doneTouched, executingConcurrent, executingSequential, groupFailed };
export { landed };

/** Every moment, in the order the work happens. */
export const ARC_MOMENTS: readonly ArcMoment[] = [
  dispatchTyping(),
  dispatchSketch(),
  proposingDispatched(),
  proposingReading(),
  proposingSlow(),
  // The four, in the order the fields settle. They sit between the wait and the
  // answer because that is where they happen.
  ...proposingFilling(),
  proposingReview(),
  approvedFrozen(),
  plannedMoment(),
  planRevisionRefused(),
  executingSequential(),
  executingConcurrent(),
  groupFailed(),
  doneTouched(),
  landed(),
];

/** One moment by name, or nothing where no builder has that name. */
export function arcMoment(name: string): ArcMoment | undefined {
  return ARC_MOMENTS.find((one) => one.name === name);
}
