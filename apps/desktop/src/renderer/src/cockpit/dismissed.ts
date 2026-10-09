// Calls the owner has dismissed for good, as opposed to put off for later. **One remembered set**, kept
// in `localStorage` across restarts, and the two ways into it behind two functions so each is swapped
// alone: a Session's waiting item goes to Fleet's `dismiss_waiting` once the window has it
// (`window.armada.dismissWaiting`), and until then is remembered here by `session_id:item_id`; a call
// Fleet raised about a state (a failing pull request, main red, a failed Check, a stuck Drone) is
// remembered by the identity of that state, so the same failure stays gone and a new one comes back.

import { useSyncExternalStore } from "react";
import type { WaitingItem } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";

import type { Item } from "../Dashboard";

const KEY = "armada.bridge.dismissed-calls";

/** What a call raised about a state is remembered by: that it is this failure, as it reads now. */
const RAISED = "call:";

function read(): ReadonlySet<string> {
  try {
    const stored: unknown = JSON.parse(window.localStorage.getItem(KEY) ?? "[]");
    return new Set(Array.isArray(stored) ? stored.filter((one): one is string => typeof one === "string") : []);
  } catch {
    return new Set();
  }
}

let held: ReadonlySet<string> | undefined;
const listeners = new Set<() => void>();

/** Every dismissed identity. Read once, then kept in step with what is written. */
export function dismissedCalls(): ReadonlySet<string> {
  held ??= read();
  return held;
}

function keep(next: ReadonlySet<string>): void {
  held = next;
  try {
    window.localStorage.setItem(KEY, JSON.stringify([...next]));
  } catch {
    // A failed write leaves it unremembered past this window, which is the honest answer for a preference.
  }
  listeners.forEach((one) => one());
}

/** The set, live: a card dismissed in this window is gone from the next render. */
export function useDismissed(): ReadonlySet<string> {
  return useSyncExternalStore(
    (onChange) => (listeners.add(onChange), () => void listeners.delete(onChange)),
    dismissedCalls,
  );
}

/** Forgets every dismissal and what was written, for a test that starts clean. */
export function forgetDismissals(): void {
  held = new Set();
  try {
    window.localStorage.removeItem(KEY);
  } catch {
    // Nothing was written to forget.
  }
  listeners.forEach((one) => one());
}

/** What a Session's waiting item is remembered by: the pair, as Fleet's route names it. */
export const waitingIdentity = (sessionId: string, itemId: string): string => `${sessionId}:${itemId}`;

/**
 * What a call is remembered by. A Session's item is its pair. A call about a state is its key and the
 * fact it states, so it is this failure: the wire serves no head commit for a pull request, so a new
 * failure is told by `forgetCleared` letting go of the old one once the state has cleared.
 */
export const identityOf = (item: Pick<Item, "key" | "fact" | "waiting">): string =>
  item.waiting === undefined ? `${RAISED}${item.key}|${item.fact}` : waitingIdentity(item.waiting.sessionId, item.waiting.item.id);

/** Whether a Session's item was dismissed. */
export const waitingDismissed = (dismissed: ReadonlySet<string>, sessionId: string, item: Pick<WaitingItem, "id">): boolean =>
  dismissed.has(waitingIdentity(sessionId, item.id));

/**
 * Lets go of a dismissed call whose state no longer raises it, so the next time it is raised it shows.
 * `raised` is every identity standing now. Only runs where the Board is read, since an empty one
 * during a resync would clear everything.
 */
export function forgetCleared(raised: ReadonlySet<string>, loaded: boolean): void {
  if (!loaded) return;
  const stale = [...dismissedCalls()].filter((one) => one.startsWith(RAISED) && !raised.has(one));
  if (stale.length === 0) return;
  keep(new Set([...dismissedCalls()].filter((one) => !stale.includes(one))));
}

/** Dismiss a call raised about a state, for good. */
export function dismissCall(item: Pick<Item, "key" | "fact" | "waiting">): void {
  keep(new Set([...dismissedCalls(), identityOf(item)]));
}

/** Dismissed, or nothing held it any more (quietly the same), or Fleet refused in these words. */
export type WaitingDismissed = { kind: "dismissed" } | { kind: "refused"; said: string };

/** What the preload carries once Sessions in Armada serves `POST /sessions/waiting/dismiss`. */
type DismissRoute = { dismissWaiting?: (dismissal: { session_id: string; item_id: string }) => Promise<{ ok: true } | { ok: false; outcome: Parameters<typeof refusalWords>[0] }> };

/**
 * Dismiss one thing a Session waits on. **Fleet's route where the window has one**, which also takes
 * it off every other window and the phone; remembered here until it does.
 */
export async function dismissWaiting(sessionId: string, itemId: string): Promise<WaitingDismissed> {
  const route = (window.armada as DismissRoute).dismissWaiting;
  if (route === undefined) {
    keep(new Set([...dismissedCalls(), waitingIdentity(sessionId, itemId)]));
    return { kind: "dismissed" };
  }
  const done = await route({ session_id: sessionId, item_id: itemId });
  if (done.ok) return { kind: "dismissed" };
  const outcome = done.outcome;
  // An item nothing holds is already gone, which is what was asked.
  if (!outcome.ok && outcome.why === "refused" && outcome.error.code === "fleet.session_waiting_unheld") return { kind: "dismissed" };
  return { kind: "refused", said: refusalWords(outcome) };
}

/** Dismiss the call in front for good, by whichever way it is kept. A refusal goes to `tell` in Fleet's words. */
export async function dismissItem(item: Pick<Item, "key" | "fact" | "waiting">, tell: ((said: string) => void) | undefined): Promise<void> {
  if (item.waiting === undefined) return dismissCall(item);
  const done = await dismissWaiting(item.waiting.sessionId, item.waiting.item.id);
  if (done.kind === "refused") tell?.(done.said);
}
