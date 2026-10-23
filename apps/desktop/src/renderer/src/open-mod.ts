// A taught Logbook entry's line names the mod that keeps it, and pressing the name asks the app to open that mod in Mods. A
// window event, as `open-retro.ts` is, because the line sits in a page that does not hold the app's navigation.

import { useEffect } from "react";

const OPEN_MOD = "armada:open-mod";

let opened: string | undefined;
const heardOpened = new Set<() => void>();

/** The mod asked for by name, which the Mods list shows open. */
export const modOpened = {
  get: (): string | undefined => opened,
  set: (name: string | undefined): void => {
    if (opened === name) return;
    opened = name;
    heardOpened.forEach((on) => on());
  },
  subscribe: (on: () => void): (() => void) => (heardOpened.add(on), () => void heardOpened.delete(on)),
};

/** The kinds of mod a Logbook entry can name: the others are taught on Bridge itself and have no entry. */
export type OpenedKind = "words" | "fleet";

/** Shows the mod open, and asks the app to go to the page that lists its kind. */
export function askToOpenMod(name: string, kind: OpenedKind): void {
  modOpened.set(name);
  window.dispatchEvent(new CustomEvent<OpenedKind>(OPEN_MOD, { detail: kind }));
}

export function useOpenModAsked(open: (kind: OpenedKind) => void): void {
  useEffect(() => {
    const heard = (event: Event) => open((event as CustomEvent<OpenedKind>).detail);
    window.addEventListener(OPEN_MOD, heard);
    return () => window.removeEventListener(OPEN_MOD, heard);
  }, [open]);
}
