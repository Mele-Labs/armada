// Claiming a pull request that no Session or Job holds: the mock behind one function. The route is
// Sessions in Armada's `POST /sessions/claim_pull_request`, body `{ number, session_id? , job_id? }`
// with exactly one of the two, answered `{ number, branch, url, holder_kind, holder_id }`, and refused,
// naming the holder, where a live Session or Job already holds it or the pull request is not open.
// The shapes here mirror that body and answer and are Bridge's own until the protocol types land;
// `attachPr` is the one thing to swap for the call. Mock only.

import { useSyncExternalStore } from "react";

import type { Owner } from "./owner";

export type ClaimPullRequest = { number: number; session_id?: string; job_id?: string };
export type PullRequestClaimed = { number: number; branch: string; url: string; holder_kind: "session" | "job"; holder_id: string };
/** A refusal, with the sentence a toast says: who holds it, or that it is not open. */
export type ClaimRefused = { refused: true; why: "held" | "not_open"; holder?: string; said: string };

/** What the mock knows of the pull request it is asked to claim. */
export type ClaimedPull = { branch: string; url: string; open: boolean };

let held: ReadonlyMap<number, Owner> = new Map();
const listeners = new Set<() => void>();

/** Every claim made in this window, by pull request number. The mock's own record. */
export const claims = (): ReadonlyMap<number, Owner> => held;

/** Forgets every claim, for a test that starts clean. */
export function forgetClaims(): void {
  held = new Map();
  listeners.forEach((one) => one());
}

/** The claims, live: a card that attached a pull request draws its owner on the next render. */
export function useClaims(): ReadonlyMap<number, Owner> {
  return useSyncExternalStore(
    (onChange) => (listeners.add(onChange), () => void listeners.delete(onChange)),
    claims,
  );
}

/** The body the route takes: exactly one of the two ids. */
export const claimBody = (number: number, owner: Pick<Owner, "kind" | "id">): ClaimPullRequest =>
  owner.kind === "session" ? { number, session_id: owner.id } : { number, job_id: owner.id };

/**
 * Claim pull request `number` for `owner`. Refused, naming the holder, where one already holds it,
 * and where the pull request is not open.
 */
export async function attachPr(number: number, owner: Owner, pull: ClaimedPull): Promise<PullRequestClaimed | ClaimRefused> {
  const body = claimBody(number, owner);
  const there = held.get(body.number);
  if (!pull.open) return { refused: true, why: "not_open", said: `Pull request #${number} is not open` };
  if (there !== undefined) return { refused: true, why: "held", holder: there.title, said: `Pull request #${number} is already held by ${there.kind === "session" ? "Session" : "Job"} ${there.title}` };
  held = new Map([...held, [number, owner]]);
  listeners.forEach((one) => one());
  return { number, branch: pull.branch, url: pull.url, holder_kind: owner.kind, holder_id: owner.id };
}

export const wasRefused = (answer: PullRequestClaimed | ClaimRefused): answer is ClaimRefused => "refused" in answer;
