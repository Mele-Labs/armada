// Attaching a pull request that no Session or Job holds: Fleet's `POST /sessions/claim_pull_request`,
// through `window.armada.claimPullRequest`. The route takes exactly one of a Session's or a Job's id,
// so the picker offers both. A claim that works shows in the Session's ledger or on the Job's row,
// which is where the card reads its owner from; one Fleet refuses says who holds it, in Fleet's
// own words.

import { refusalWords } from "@armada/screens/src/refusal-words";

import type { Owner } from "./owner";

/** Attached, or refused in these words. */
export type Attached = { attached: true } | { attached: false; said: string };

export async function attachPr(number: number, to: Pick<Owner, "kind" | "id">): Promise<Attached> {
  const done = await window.armada.claimPullRequest({ number, ...(to.kind === "job" ? { job_id: to.id } : { session_id: to.id }) });
  return done.ok ? { attached: true } : { attached: false, said: refusalWords(done.outcome) };
}
