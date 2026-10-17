// Attaching a pull request that no Session holds: Fleet's `POST /sessions/claim_pull_request`,
// through `window.armada.claimPullRequest`. A claim that works shows in the Session's ledger, which is
// where the card reads its owner from; one Fleet refuses says who holds it, in Fleet's own words.

import { refusalWords } from "@armada/screens/src/refusal-words";

import type { Owner } from "./owner";

/** Attached, or refused in these words. */
export type Attached = { attached: true } | { attached: false; said: string };

export async function attachPr(number: number, to: Pick<Owner, "id">): Promise<Attached> {
  const done = await window.armada.claimPullRequest({ number, session_id: to.id });
  return done.ok ? { attached: true } : { attached: false, said: refusalWords(done.outcome) };
}
