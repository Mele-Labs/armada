// Where the open Job's state and name go while `&jobname=notch` is on (`flight-take.ts`): a place in the title bar's centre notch, after the Places, that `TabBar` draws and the Job's head
// portals its name into. Held here because the two are far apart in the tree. A mock of 26 Oct 2026.

import { useSyncExternalStore } from "react";

let slot: HTMLElement | null = null;
const listeners = new Set<() => void>();

/** The ref of the place, set by `TabBar` as it is drawn and cleared as it goes. */
export function setTitleNotch(element: HTMLElement | null): void {
  if (slot === element) return;
  slot = element;
  for (const notify of listeners) notify();
}

const subscribe = (notify: () => void) => {
  listeners.add(notify);
  return () => {
    listeners.delete(notify);
  };
};

/** The place in the title bar, or none where there is no title bar. */
export const useTitleNotch = (): HTMLElement | null => useSyncExternalStore(subscribe, () => slot);
