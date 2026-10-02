// A Job's title as one line of plain words.
//
// **Until the proposer's title lands, the title is the request as it was
// typed** — `crates/fleet/src/proposing.rs`, `Settled::title` — and a request
// is written in markdown. A Job the proposer never titled keeps it: a stopped
// call is `proposing -> killed`, a declined one `proposing -> escalated`. So
// every place that draws the title as a line reads it through here, and the
// Board row does not show `**` where a person wrote bold.
//
// **Cached, because a parse costs about 0.4 ms** and the Board redraws every
// row on its clock. A session's titles are few, so the cache is not bounded.

import { proseText } from "@armada/components";
import type { JobSummary } from "@armada/protocol";

const PLAIN = new Map<string, string>();

/** The title, with none of the markdown it may have been typed in. */
export function titleOf(job: Pick<JobSummary, "title">): string {
  let plain = PLAIN.get(job.title);
  if (plain === undefined) {
    plain = proseText(job.title);
    PLAIN.set(job.title, plain);
  }
  return plain;
}
