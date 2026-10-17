// A way for a mark on a row, anywhere in the window, to open the other holder of a write clash: its
// Session. A window event, as `open-session.ts` is, because the row must not import `App`.

import { useEffect } from "react";
import type { ClashHolderRef } from "@armada/components";

const OPEN_HOLDER = "armada:open-holder";

export function askToOpenHolder(holder: ClashHolderRef): void {
  window.dispatchEvent(new CustomEvent<ClashHolderRef>(OPEN_HOLDER, { detail: holder }));
}

/** Opens a Session when asked to. Main is not a holder there is anything to open. */
export function useOpenHolderAsked(open: { session: (id: string) => void }): void {
  useEffect(() => {
    const heard = (event: Event): void => {
      const holder = (event as CustomEvent<ClashHolderRef>).detail;
      if (holder.kind === "session") open.session(holder.id);
    };
    window.addEventListener(OPEN_HOLDER, heard);
    return () => window.removeEventListener(OPEN_HOLDER, heard);
  }, [open.session]);
}
