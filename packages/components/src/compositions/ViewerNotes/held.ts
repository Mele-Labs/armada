import { useSyncExternalStore } from "react";

import type { HeldNote } from "./compose";

/**
 * The notes held on each sketch, file or picture until they are sent, by the item's key, and the Job picked for them
 * where the viewer is in none. **In memory only**: closing a viewer and opening it again finds them; a reload does not.
 */
export type HeldOn = { notes: readonly HeldNote[]; job?: string };

const NONE: HeldOn = { notes: [] };
const held = new Map<string, HeldOn>();
const listeners = new Set<() => void>();

export function heldOn(key: string): HeldOn {
  return held.get(key) ?? NONE;
}

export function setHeldOn(key: string, next: (was: HeldOn) => HeldOn): void {
  const now = next(heldOn(key));
  if (now.notes.length === 0 && now.job === undefined) held.delete(key);
  else held.set(key, now);
  listeners.forEach((one) => one());
}

const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => void listeners.delete(listener);
};

export function useHeldOn(key: string | undefined): HeldOn {
  return useSyncExternalStore(subscribe, () => (key === undefined ? NONE : heldOn(key)));
}
