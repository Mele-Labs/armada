// Draws a floating sheet over the whole app, out of the pane that opened it. A Session's pane is glass (`backdrop-filter`), which makes it
// the containing block of anything `position: fixed` inside it, so a sheet opened from the pane was cut to the pane's box. Drawn on the body
// it takes the window, as a sheet is drawn everywhere else. Focus goes back to what had it when the sheet opened, so Esc leaves the view
// under it as it was: the row that was pressed is still where the arrows start from.

import { useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";

/**
 * `held` draws into a holder of its own, put on the body as this mounts, so a sheet inside that closes and opens again
 * keeps its place among the body's layers rather than going on top of whatever opened since: the Loadout, stepping
 * aside for a file's Preview and back under that file.
 */
export function OverTheApp({ open, held = false, children }: { open: boolean; held?: boolean; children: ReactNode }) {
  const from = useRef<HTMLElement | null>(null);
  const [holder] = useState(() => (held ? document.createElement("div") : null));
  useLayoutEffect(() => {
    if (holder === null) return;
    document.body.appendChild(holder);
    return () => holder.remove();
  }, [holder]);
  // Layout effects run before the sheet's own passive effect moves focus into it, so this reads where focus was.
  useLayoutEffect(() => {
    if (!open) return;
    const active = document.activeElement;
    from.current = active instanceof HTMLElement && active !== document.body ? active : null;
    return () => {
      const back = from.current;
      from.current = null;
      if (back?.isConnected === true) window.setTimeout(() => back.focus(), 0);
    };
  }, [open]);
  return createPortal(children, holder ?? document.body);
}
