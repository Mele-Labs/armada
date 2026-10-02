// A step's gaming check, read once for the rail and the step panel. #1079.
//
// **Its own reading, never a tier's.** The gaming check is counted in neither
// Checks nor Judge on purpose — a flag is not a verdict, and a recount would
// make a green tier read red — so nothing here touches `gates.ts`' counts.
//
// **A flag a second reading cleared is still a flag**, and it never holds a
// step: it is drawn in the panel as cleared, with why, and nowhere else.

import type { Flagged, StepDetail } from "@armada/protocol";

import { onlyCurrentAttempt } from "./facts";

/** One attempt's flags, split into those that hold the step and those cleared. */
export type FlagsRead = { held: Flagged[]; cleared: Flagged[] };

/** The newest attempt's flags, or the rows a caller already narrowed. */
export function flagsOf(step: StepDetail, rows?: readonly Flagged[]): FlagsRead {
  const flags = rows ?? onlyCurrentAttempt(step.flagged);
  return {
    held: flags.filter((flag) => flag.cleared === undefined),
    cleared: flags.filter((flag) => flag.cleared !== undefined),
  };
}
