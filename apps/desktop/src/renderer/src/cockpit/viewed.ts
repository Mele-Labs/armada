// When the owner was last in a Session, per Session, kept in `localStorage` so it survives a restart. The
// Cockpit raises a call for a Session's waiting item only if he has not been in that Session since the item
// began waiting (`touchedAt` in `waiting.ts`); this is the "been in" that is Bridge's to know. A Session open
// on the Sessions page, or in its walk window, with the window visible, is being viewed now: its time is
// "now" until he leaves, and again when he does. Another window's writes arrive on the `storage` event.

import { useEffect, useSyncExternalStore } from "react";

const KEY = "armada.bridge.session-viewed";

function read(): Record<string, number> {
  try {
    const stored: unknown = JSON.parse(window.localStorage.getItem(KEY) ?? "{}");
    if (typeof stored !== "object" || stored === null || Array.isArray(stored)) return {};
    return Object.fromEntries(Object.entries(stored).filter((one): one is [string, number] => typeof one[1] === "number"));
  } catch {
    return {};
  }
}

let held: Record<string, number> | undefined;
let version = 0;
/** Sessions on screen in this window, and how many places draw each. */
const viewing = new Map<string, number>();
const listeners = new Set<() => void>();

const changed = (): void => {
  version += 1;
  listeners.forEach((one) => one());
};

const visible = (): boolean => typeof document === "undefined" || document.visibilityState !== "hidden";

/** The last instant he was in this Session: now, while it is on screen; 0 where he never was. */
export function viewedAt(sessionId: string): number {
  if ((viewing.get(sessionId) ?? 0) > 0 && visible()) return Date.now();
  held ??= read();
  return held[sessionId] ?? 0;
}

/** Whether this Session is on screen in this window now, with the window visible. */
export const viewingNow = (sessionId: string): boolean => (viewing.get(sessionId) ?? 0) > 0 && visible();

/** Record that he was in this Session at `at`. */
export function markViewed(sessionId: string, at: number = Date.now()): void {
  held ??= read();
  held = { ...held, [sessionId]: Math.max(at, held[sessionId] ?? 0) };
  try {
    window.localStorage.setItem(KEY, JSON.stringify(held));
  } catch {
    // A failed write leaves it unremembered past this window, which is the honest answer for a preference.
  }
  changed();
}

/** Forgets every Session's time and what was written, for a test that starts clean. */
export function forgetViewed(): void {
  held = {};
  viewing.clear();
  try {
    window.localStorage.removeItem(KEY);
  } catch {
    // Nothing was written to forget.
  }
  changed();
}

function subscribe(onChange: () => void): () => void {
  listeners.add(onChange);
  return () => void listeners.delete(onChange);
}

/** Changes whenever any Session's time does, so a list built from `viewedAt` knows to be built again. */
export function useViewedVersion(): number {
  return useSyncExternalStore(subscribe, () => version);
}

/**
 * Marks the Session as being in, while this is mounted with a Session and the window is visible. Marks
 * it on arriving and on leaving (or hiding, or closing), so the time spent there counts up to the last
 * moment he was.
 */
export function useViewing(sessionId: string | undefined): void {
  useEffect(() => {
    if (sessionId === undefined) return;
    viewing.set(sessionId, (viewing.get(sessionId) ?? 0) + 1);
    if (visible()) markViewed(sessionId);
    else changed();
    const leave = (): void => markViewed(sessionId);
    const toggled = (): void => (visible() ? markViewed(sessionId) : leave());
    document.addEventListener("visibilitychange", toggled);
    window.addEventListener("beforeunload", leave);
    return () => {
      document.removeEventListener("visibilitychange", toggled);
      window.removeEventListener("beforeunload", leave);
      const left = (viewing.get(sessionId) ?? 1) - 1;
      if (left > 0) viewing.set(sessionId, left);
      else viewing.delete(sessionId);
      if (visible()) markViewed(sessionId);
    };
  }, [sessionId]);
}

// Another Bridge window (a walk's Session panel) wrote a time: read it again.
if (typeof window !== "undefined") {
  window.addEventListener("storage", (event) => {
    if (event.key !== KEY && event.key !== null) return;
    held = read();
    changed();
  });
}
