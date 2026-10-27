// What a Session is waiting on the owner for, as one list of items: the wire's own
// (`SessionRecord.waiting_for`, which the draft carries as `Session.waitingFor`) and, where a Session
// carries none, the same items read off the pending ask the draft already holds (`Session.asked`: a
// permission, or the agent's own questions). **One adapter**: `callsFromWaiting` takes either. Pure.

import type { WaitingItem } from "@armada/protocol";
import type { Session, SessionAnswer } from "@armada/screens/src/draft/sessions";

/**
 * An option as the card draws it. The wire's is a label alone; the agent's own line under it rides
 * only on an item read off the pending ask, and is drawn where it is there.
 */
export type Described = { label: string; description?: string };

export type WaitingCall = { session: Session; item: WaitingItem };

/** What a permission offers, as the labels the wire's permission items carry. */
const PERMISSION: Record<SessionAnswer, string> = {
  allow_once: "Allow once",
  allow_and_remember: "Allow and remember",
  refuse: "Refuse",
  always_allow: "Always allow",
  always_block: "Always block",
  allow_for_session: "Always allow for this job",
  allow_like_this_for_session: "Allow calls like this for this job",
};
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

/**
 * What holds a turn open on the owner: an agent mid-turn on one of these is waiting, not at work. A recorded plan and a
 * judge's concern call too while the turn goes on: the plan is the agent's to start from, and a judge never blocks.
 * A full pool (`slot_choice`) holds the agent's first write until he answers, and the delivery question (`delivery`)
 * holds the agent at its start. A `fleet_call` is a notice a taught rule made a call: it holds nothing, and it calls while the
 * turn goes on because calling him is all it is for. A `draft` waits on his OK however the turn goes, as a question does. The done
 * ask on his project tasks (`task_done`) calls too: a Job that opened a pull request, or said it was done, may still be at work,
 * and the tasks it holds are waiting on him either way.
 */
const BLOCKING = new Set(["ask_card", "permission", "window_ask", "plan", "judge", "slot_choice", "delivery", "fleet_call", "draft", "task_done"]);

/** The latest message from the owner on the Session's thread, as an instant; 0 where he has sent none. */
export function lastOwnerMessageAt(session: Session): number {
  let latest = 0;
  for (const row of session.rows) if (row.kind === "message" && row.from.kind === "you") latest = Math.max(latest, Date.parse(row.at) || 0);
  return latest;
}

/** The last instant he was in the Session: viewing it, or sending it a message, whichever is later. */
export const touchedAt = (session: Session, viewedAt: (sessionId: string) => number): number => Math.max(viewedAt(session.id), lastOwnerMessageAt(session));

/**
 * Every item every live Session waits on, the one that began first leading. A Session at work shows
 * only what its turn is stopped on: its question or permission reads `working` until it is answered.
 * **An item raises a call only if he has not been in its Session since it began waiting**: if he was,
 * he saw it there, and it stays on the Session's own list alone. An item re-posted under the same id
 * keeps its `since`, so it does not call again; a new one does. `viewedAt` is when he last viewed a Session.
 * **A reply that only tells raises none**: it asks nothing of him, and the lists mark it unread. A reply that asks is a
 * `replied` item and calls like any other.
 */
export function callsFromWaiting(sessions: readonly Session[], viewedAt: (sessionId: string) => number = () => 0): WaitingCall[] {
  return sessions
    // A paused Job is parked by him: nothing it waits on calls until he resumes it.
    .filter((session) => session.dead === undefined && session.paused !== true)
    .flatMap((session) => {
      const touched = touchedAt(session, viewedAt);
      return waitingOf(session)
        .filter((item) => session.turn.state !== "working" || BLOCKING.has(item.source))
        .filter((item) => touched === 0 || Date.parse(item.since) > touched)
        .map((item) => ({ session, item }));
    })
    .sort((a, b) => Date.parse(a.item.since) - Date.parse(b.item.since) || a.item.id.localeCompare(b.item.id));
}

/** What a Session's call is called, whichever builder raises it: the item's source decides, so a call keeps its label after the Session has been viewed. */
export const callKindOf = (source: string): string => (source === "ask_card" || source === "window_ask" ? "Job question" : source === "walk" ? "Job walk" : "Job");

/** The Session a key names, whether it is the Session's own tile (`session:<id>`) or one of its calls (`session:<id>:<item>`). */
export const sessionIdOf = (key: string): string | undefined => (key.startsWith("session:") ? key.split(":")[1] : undefined);
