// The window listener a list's keyboard needs, shared by every list that takes keys.
//
// **`keys.ts` owns the map — what a key means.** This file owns none of that; it is the listener alone.

import { useEffect, useRef } from "react";

/**
 * The window listener every list keyboard needs: registered once, reading the latest handler out of
 * a ref so a clock-driven re-render does not churn the subscription.
 *
 * **Refuses every press while a dialog is open.** A confirmation is `App`'s, outside whichever list
 * component is listening, so a window listener would otherwise act on a row behind the dialog a
 * person is answering.
 */
export function useListKeydown(press: (event: KeyboardEvent) => void): void {
  const latest = useRef(press);
  useEffect(() => {
    latest.current = press;
  });
  useEffect(() => {
    const listen = (event: KeyboardEvent): void => {
      if (document.querySelector('[role="dialog"], [role="alertdialog"]') !== null) return;
      latest.current(event);
    };
    window.addEventListener("keydown", listen);
    return () => window.removeEventListener("keydown", listen);
  }, []);
}
