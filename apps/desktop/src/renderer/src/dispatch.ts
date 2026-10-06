// The Job proposer's one call, and the only place this app makes it.
//
// **Its own file for the reason `palette.ts` is one**: it is a subject rather
// than a line of wiring, and `App.tsx` is already over the length the gate warns
// at. What is here is the preload entry and the one reading of its answer.
//
// # It is fired and left, since a dispatched request is a Job
//
// The press leaves the composer — `job-statuses.toml`, `proposing` — so nothing
// is on screen waiting for this to come back. What the caller still does with
// the answer is tell somebody where Fleet declined, and raise a failure where
// the call could not be made: `answeredAs` decides which, and neither needs the
// workflow roster or Bridge's identity that this used to be handed.

import { answeredAs } from "@armada/jobs";
import type { Answered } from "@armada/jobs";
import type { StagedAttachment } from "@armada/protocol";

/**
 * Read a request, and answer with the one thing left to do about it.
 *
 * **No guard here.** Nothing about this call is idempotent, and the form is
 * what stops a second press — see `DispatchJob` in `@armada/jobs`. A guard
 * in two places is two answers about whether a request went out.
 *
 * `repository` is the root New job's own ask answered, on All — #959: the
 * Board stays there while composing, so the request names what was answered
 * rather than a pick that never moved. `null` where a repository was already
 * picked before the composer opened.
 */
export async function proposeRequest(
  request: string,
  attachments: readonly StagedAttachment[],
  repository: string | null = null,
): Promise<Answered> {
  return answeredAs(await window.armada.proposeFromRequest(request, attachments, repository));
}
