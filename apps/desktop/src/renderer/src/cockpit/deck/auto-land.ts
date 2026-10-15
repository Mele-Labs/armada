// The Sessions the owner turned Auto land on for, kept across a restart. `localStorage`, as `dismissed.ts` keeps the calls he
// dismissed: Fleet has no operation for it, and the instruction itself goes to the agent as a message, so the tile only has to
// remember what the owner last told it. Read from storage on each look and parsed when the text changes, so a window that
// forgets how it was read (a test, a cleared profile) forgets this too.

import { useSyncExternalStore } from "react";

const KEY = "armada.bridge.session-auto-land";

let raw: string | null | undefined;
let held: ReadonlySet<string> = new Set();
const listeners = new Set<() => void>();

function stored(): string | null {
  try {
    return window.localStorage.getItem(KEY);
  } catch {
    return null;
  }
}

function snapshot(): ReadonlySet<string> {
  const now = stored();
  if (now !== raw) {
    raw = now;
    try {
      const parsed: unknown = JSON.parse(now ?? "[]");
      held = new Set(Array.isArray(parsed) ? parsed.filter((one): one is string => typeof one === "string") : []);
    } catch {
      held = new Set();
    }
  }
  return held;
}

/** Whether Auto land is on for this Session. */
export const autoLandOn = (id: string): boolean => snapshot().has(id);

/** Turns it on or off for one Session. A failed write leaves it unremembered past this window. */
export function setAutoLand(id: string, on: boolean): void {
  const next = new Set(snapshot());
  if (on) next.add(id);
  else next.delete(id);
  try {
    window.localStorage.setItem(KEY, JSON.stringify([...next]));
  } catch {
    // Unremembered.
  }
  raw = undefined;
  snapshot();
  listeners.forEach((one) => one());
}

/** Whether it is on, live: the tile reads it. */
export function useAutoLand(id: string): boolean {
  return useSyncExternalStore(
    (onChange) => (listeners.add(onChange), () => void listeners.delete(onChange)),
    () => autoLandOn(id),
  );
}
