// The default folded reads. Apart from `base.ts`, the shared core, because the type
// is a Job detail's.

import type { FoldedReads } from "../../JobDetail";
import { NO_DIFF, NO_EVIDENCE, NO_FOOTPRINT, NO_REMARKS } from "./base";

/** The default folded reads — every read `JobDetail.recorded` needs, empty. */
export function foldedReads(over: Partial<FoldedReads> = {}): FoldedReads {
  return { footprint: NO_FOOTPRINT, handed: { state: "none" }, evidence: NO_EVIDENCE, diff: NO_DIFF, remarks: NO_REMARKS, ...over };
}
