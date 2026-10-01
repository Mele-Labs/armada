// The wait a Job at `proposing` is in, built for the slot inside Overview's
// lead — `leadOf` says *A model is reading the request*, and this is how far
// that call has got.
//
// **Its own file rather than another reading in `tab-overview.tsx`**, which is
// already at the length the gate refuses. One subject: the one status whose
// lead is about a model call rather than about anything a step holds.

import { ProposerWait } from "@armada/components";
import type { JobSummary, ProposalInFlight } from "@armada/protocol";
import type { ReactNode } from "react";

import { PROPOSAL_IS_SLOW, watchOf } from "./proposal";

/**
 * The status a dispatched request stands at until the proposer answers. A wire
 * value, for `lead.ts`'s own reason — spelled there too, and the two are the
 * same key into the same registry rather than a shared constant nothing else
 * would read.
 */
const BEING_PROPOSED = "proposing";

/**
 * The wait, or nothing where the Job is not being proposed.
 *
 * **Drawn even where the window holds no reading of the call.** A Job at
 * `proposing` is one somebody dispatched and walked away from, so a window
 * reopened on it has an address and no call of its own — `ProposerWait` says
 * the proposer is reading and offers the stop, which is true either way.
 */
export function proposerWaitOf(
  job: JobSummary,
  proposing: ProposalInFlight | null,
  now: number,
  onStop?: () => void,
): ReactNode {
  if (job.status !== BEING_PROPOSED) return undefined;
  const watch = watchOf(proposing, now);
  return (
    <ProposerWait
      {...(watch === null ? {} : { watch })}
      {...(onStop === undefined ? {} : { onStop })}
      slowAfterMs={PROPOSAL_IS_SLOW}
    />
  );
}
