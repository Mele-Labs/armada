// The window listener a list's keyboard needs, shared by every list that takes keys.
//
// **`keys.ts` owns the map — what a key means.** This file owns none of that; it is the listener alone.

import { useEffect, useRef } from "react";

/** What an open popup leaves on the page: a menu's panel, or the trigger or field of a menu, list or dialog that is showing. */
const POPUP = '[role="menu"], [aria-haspopup][aria-expanded="true"], [role="combobox"][aria-expanded="true"]';

/**
 * Whether a menu, list or other popup is up, wherever focus is. **Esc closes the innermost thing,
 * so a listener that answers Esc by going back stands down while this holds.** It reads the page and
 * not the key's target: a press on the menu's padding drops focus to the body, and a target check
 * then lets the Esc through to whatever sits under the menu.
 */
export function popupIsOpen(): boolean {
  return document.querySelector(POPUP) !== null;
}

/**
 * The window listener every list keyboard needs: registered once, reading the latest handler out of
 * a ref so a clock-driven re-render does not churn the subscription.
 *
 * **Refuses every press while a dialog is open.** A confirmation is `App`'s, outside whichever list
 * component is listening, so a window listener would otherwise act on a row behind the dialog a
 * person is answering. **Esc is refused while a popup is up** (`popupIsOpen`): it is the popup's.
 */
export function useListKeydown(press: (event: KeyboardEvent) => void): void {
  const latest = useRef(press);
  useEffect(() => {
    latest.current = press;
  });
  useEffect(() => {
    const listen = (event: KeyboardEvent): void => {
      if (document.querySelector('[role="dialog"], [role="alertdialog"]') !== null) return;
      if (event.key === "Escape" && popupIsOpen()) return;
      latest.current(event);
    };
    window.addEventListener("keydown", listen);
    return () => window.removeEventListener("keydown", listen);
  }, []);
}
