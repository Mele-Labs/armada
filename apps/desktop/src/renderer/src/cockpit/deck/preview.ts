// The preview switch: the flight deck in place of the classic Cockpit and its rail. **The switch is the mount.** With it on,
// the window draws no rail, the places in the title bar and the deck as the Cockpit's body; with it off nothing of the deck is
// mounted, and the classic Cockpit and the rail are untouched. A remembered view setting like the deck's own (`store.ts`),
// held in the module so the title bar, the body and the palette entry agree. #2301.

import { useSyncExternalStore } from "react";

import { DECK_PREVIEW_KEY, DECK_SYSTEM_KEY, readRemembered, writeRemembered } from "../../remembered-views";

const onNamed = (value: string | null): "on" | "off" => (value === "on" ? "on" : "off");

let on = readRemembered(DECK_PREVIEW_KEY, onNamed) === "on";
const listeners = new Set<() => void>();
const subscribe = (listener: () => void) => (listeners.add(listener), () => void listeners.delete(listener));

export const deckPreview = (): boolean => on;
export const useDeckPreview = (): boolean => useSyncExternalStore(subscribe, deckPreview);

/** Turns the preview on or off, and remembers it. */
export function setDeckPreview(next: boolean): void {
  if (on === next) return;
  on = next;
  writeRemembered(DECK_PREVIEW_KEY, next ? "on" : "off");
  listeners.forEach((listener) => listener());
}

/** The palette's one row for the switch: the deck when it is off, the classic Cockpit when it is on. */
export const previewLabel = (now: boolean): "Flight deck" | "Classic Cockpit" => (now ? "Classic Cockpit" : "Flight deck");

/** The System page last used, which the System tab opens on. Kit where none has been. */
export const systemPageOf = (pages: readonly string[], first: string): string => {
  const kept = readRemembered(DECK_SYSTEM_KEY, (value) => value ?? "");
  return pages.includes(kept) ? kept : first;
};
export const rememberSystemPage = (surface: string): void => writeRemembered(DECK_SYSTEM_KEY, surface);
