// Calls the owner has dismissed for good, as opposed to put off for later. A Session's waiting item is
// dismissed on Fleet (`window.armada.dismissWaiting`), which drops it from the Session's list for good.
// A call Fleet raised about a state (a failing pull request, main red, a failed Check, a stuck Drone)
// has no such route, so it is remembered here, in `localStorage` across restarts, by the identity of
// that state: the same failure stays gone and a new one comes back.

import { useSyncExternalStore } from "react";
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

/**
 * What a call about a state is remembered by: its key and the fact it states, so it is this failure.
 * The wire serves no head commit for a pull request, so a new failure is told by `forgetCleared`
 * letting go of the old one once the state has cleared.
 */
export const identityOf = (item: Pick<Item, "key" | "fact">): string => `${RAISED}${item.key}|${item.fact}`;

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
export function dismissCall(item: Pick<Item, "key" | "fact">): void {
  keep(new Set([...dismissedCalls(), identityOf(item)]));
}

/** Dismissed, or nothing held it any more (quietly the same), or Fleet refused in these words. */
export type WaitingDismissed = { kind: "dismissed" } | { kind: "refused"; said: string };

/** Dismiss one thing a Session waits on, on Fleet, which also takes it off every other window and the phone. */
export async function dismissWaiting(sessionId: string, itemId: string): Promise<WaitingDismissed> {
  const done = await window.armada.dismissWaiting({ session_id: sessionId, item_id: itemId });
  if (done.ok) return { kind: "dismissed" };
  const outcome = done.outcome;
  // An item nothing holds is already gone, which is what was asked.
  if (!outcome.ok && outcome.why === "refused" && outcome.error.code === "fleet.session_waiting_unheld") return { kind: "dismissed" };
  return { kind: "refused", said: refusalWords(outcome) };
}

/** Dismiss the call in front for good, by whichever way it is kept. A refusal comes back in Fleet's words. */
export async function dismissItem(item: Pick<Item, "key" | "fact" | "waiting">): Promise<WaitingDismissed> {
  if (item.waiting === undefined) {
    dismissCall(item);
    return { kind: "dismissed" };
  }
  return dismissWaiting(item.waiting.sessionId, item.waiting.item.id);
}
