// What a Session is waiting on the owner for, as one list of items: the wire's own
// (`SessionRecord.waiting_for`, which the draft carries as `Session.waitingFor`) and, where a Session
// carries none, the same items read off the pending ask the draft already holds (`Session.asked`: a
// permission, or the agent's own questions). **One adapter**: `callsFromWaiting` takes either. Pure.

import type { WaitingItem } from "@armada/protocol";
import type { Session, SessionAnswer } from "@armada/screens/src/draft/sessions";

import { waitingDismissed } from "./dismissed";

/**
 * An option as the card draws it. The wire's is a label alone; the agent's own line under it rides
 * only on an item read off the pending ask, and is drawn where it is there.
 */
export type Described = { label: string; description?: string };

export type WaitingCall = { session: Session; item: WaitingItem };

/** What a permission offers, as the labels the wire's permission items carry. */
const PERMISSION: Record<SessionAnswer, string> = { allow_once: "Allow once", allow_and_remember: "Allow and remember", refuse: "Refuse" };
const PERMISSION_OF: Record<string, SessionAnswer> = { "Allow once": "allow_once", "Allow and remember": "allow_and_remember", Refuse: "refuse" };

/** The answer a permission option's label stands for, or none where it is not one of the three. */
export const permissionOf = (label: string): SessionAnswer | undefined => PERMISSION_OF[label];

/** The wire's items on the Session where it serves them; the pending ask, read as the same items, where it does not. */
export function waitingOf(session: Session): WaitingItem[] {
  if (session.waitingFor !== undefined) return [...session.waitingFor];
  const asked = session.asked;
  if (asked === undefined) return [];
  const since = session.lastTurnAt ?? new Date(0).toISOString();
  if (asked.questions !== undefined && asked.questions.length > 0) {
    return asked.questions.map((one, at): WaitingItem => ({
      id: `ask:${asked.call ?? session.id}:${at}`,
      text: one.question,
      since,
      source: "ask_card",
      options: one.options.map((option): Described => ({ label: option.label, ...(option.description === "" ? {} : { description: option.description }) })),
    }));
  }
  const offers = asked.offers ?? (["allow_once", "refuse"] as const);
  return [{ id: `perm:${asked.call ?? session.id}`, text: asked.command, since, source: "permission", options: offers.map((one) => ({ label: PERMISSION[one] })) }];
}

/** Every item every live Session waits on, the one that began first leading, less those dismissed for good. */
export function callsFromWaiting(sessions: readonly Session[], dismissed: ReadonlySet<string> = new Set()): WaitingCall[] {
  return sessions
    .filter((session) => session.dead === undefined && session.turn.state !== "working")
    .flatMap((session) => waitingOf(session).filter((item) => !waitingDismissed(dismissed, session.id, item)).map((item) => ({ session, item })))
    .sort((a, b) => Date.parse(a.item.since) - Date.parse(b.item.since) || a.item.id.localeCompare(b.item.id));
}

/** The Session a key names, whether it is the Session's own tile (`session:<id>`) or one of its calls (`session:<id>:<item>`). */
export const sessionIdOf = (key: string): string | undefined => (key.startsWith("session:") ? key.split(":")[1] : undefined);
