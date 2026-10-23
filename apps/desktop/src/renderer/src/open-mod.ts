// A taught Logbook entry's line names the rules mod that keeps it, and pressing the name asks the app to open that mod in Mods. A
// window event, as `open-retro.ts` is, because the line sits in a page that does not hold the app's navigation.

import { useEffect } from "react";

import { taughtRules } from "./teach";

const OPEN_MOD = "armada:open-mod";

/** Shows the rules mod open, and asks the app to go to the page that lists it. */
export function askToOpenMod(name: string): void {
  taughtRules.open(name);
  window.dispatchEvent(new Event(OPEN_MOD));
}

export function useOpenModAsked(open: () => void): void {
  useEffect(() => {
    window.addEventListener(OPEN_MOD, open);
    return () => window.removeEventListener(OPEN_MOD, open);
  }, [open]);
}
