// What a Session is waiting on the owner for, as one list of items: the shape Sessions in Armada gives
// the draft (`Session.waitingFor`, from `SessionRecord.waiting_for`), and, until that serves, the same
// items read off the pending ask the draft already carries (`Session.asked`: a permission, or the
// agent's own questions). **One adapter, so the swap is one line**: `callsFromWaiting` takes either.
// The types here mirror the wire's items and are Bridge's own until its protocol types land. Pure.

import type { Session, SessionAnswer } from "@armada/screens/src/draft/sessions";

/** One answer a waiting item offers. `description` is the agent's own line under it. */
export type WaitingOption = { label: string; description?: string };

/** What a Session waits on, with a stable id: `ask:<call>`, `perm:<call>`, `walk:<url>`. */
export type WaitingItem = {
  id: string;
  text: string;
  /** ISO. Items stand in the order they began. */
  since: string;
  source: "agent" | "ask_card" | "walk" | "permission";
  /** What answering does where it is more than a reply: approve a walk, answer, approve a pull request, run. */
  act?: { kind: "walk" | "answer" | "approve_pr" | "run"; target: string };
  options?: readonly WaitingOption[];
};

export type WaitingCall = { session: Session; item: WaitingItem };

/** What a permission offers, as the labels the wire's permission items carry. */
const PERMISSION: Record<SessionAnswer, string> = { allow_once: "Allow once", allow_and_remember: "Allow and remember", refuse: "Refuse" };
const PERMISSION_OF: Record<string, SessionAnswer> = { "Allow once": "allow_once", "Allow and remember": "allow_and_remember", Refuse: "refuse" };

/** The answer a permission option's label stands for, or none where it is not one of the three. */
export const permissionOf = (label: string): SessionAnswer | undefined => PERMISSION_OF[label];

/** The wire's items on the Session where it serves them; the pending ask, read as the same items, where it does not. */
export function waitingOf(session: Session): WaitingItem[] {
  const served = (session as Session & { waitingFor?: readonly WaitingItem[] }).waitingFor;
  if (served !== undefined) return [...served];
  const asked = session.asked;
  if (asked === undefined) return [];
  const since = session.lastTurnAt ?? new Date(0).toISOString();
  if (asked.questions !== undefined && asked.questions.length > 0) {
    return asked.questions.map((one, at): WaitingItem => ({
      id: `ask:${asked.call ?? session.id}:${at}`,
      text: one.question,
      since,
      source: "ask_card",
      options: one.options.map((option) => ({ label: option.label, ...(option.description === "" ? {} : { description: option.description }) })),
    }));
  }
  const offers = asked.offers ?? (["allow_once", "refuse"] as const);
  return [{ id: `perm:${asked.call ?? session.id}`, text: asked.command, since, source: "permission", options: offers.map((one) => ({ label: PERMISSION[one] })) }];
}

/** Every item every live Session waits on, the one that began first leading. */
export function callsFromWaiting(sessions: readonly Session[]): WaitingCall[] {
  return sessions
    .filter((session) => session.dead === undefined && session.turn.state !== "working")
    .flatMap((session) => waitingOf(session).map((item) => ({ session, item })))
    .sort((a, b) => Date.parse(a.item.since) - Date.parse(b.item.since) || a.item.id.localeCompare(b.item.id));
}

/** The Session a key names, whether it is the Session's own tile (`session:<id>`) or one of its calls (`session:<id>:<item>`). */
export const sessionIdOf = (key: string): string | undefined => (key.startsWith("session:") ? key.split(":")[1] : undefined);
