// The System entry last picked, which the System tab opens on. Skills where none has been. It lives in a
// module because a palette row, a key and the sidebar all move it, and a window can be on System before
// the page that draws it is mounted.

import { useSyncExternalStore } from "react";

import { DECK_SYSTEM_KEY, readRemembered, writeRemembered } from "./remembered-views";
import { entryOf, FIRST_ENTRY } from "./system-entries";

const listeners = new Set<() => void>();

/** The entry last picked. A stored word that is no entry (an older build kept a surface here) is Skills. */
export function systemEntryNow(): string {
  const kept = readRemembered(DECK_SYSTEM_KEY, (value) => value ?? "");
  return entryOf(kept) === undefined ? FIRST_ENTRY : kept;
}

export function openSystemEntry(id: string): void {
  if (entryOf(id) === undefined || id === systemEntryNow()) return;
  writeRemembered(DECK_SYSTEM_KEY, id);
  listeners.forEach((on) => on());
}

export function useSystemEntry(): [string, (id: string) => void] {
  const now = useSyncExternalStore(
    (on) => {
      listeners.add(on);
      return () => void listeners.delete(on);
    },
    systemEntryNow,
    systemEntryNow,
  );
  return [now, openSystemEntry];
}
