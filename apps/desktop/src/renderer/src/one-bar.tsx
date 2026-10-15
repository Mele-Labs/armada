// The title bar's one bar: search and dispatch in one field, proposal D (the owner, 10 Oct 2026). It
// replaced the Dashboard's dispatch bar and the title bar's Dispatch button; the Cockpit has no bar of its own now.
// `n` and ⌘N no longer open the palette: they open the dispatch panel on the draft, in one press, from every
// surface. The palette is search, and a dispatch row chosen in it (↵ on the row, ⌘↵, a pasted link) moves the
// words into that same panel, in the kind Tab chose. The panel, its draft and the sends are `dispatch-panel/`.

import { useEffect, useRef } from "react";
import { isPressed, type PaletteDispatch } from "@armada/components";
import { holdsText } from "@armada/screens/src/keys";

import type { Dispatching } from "./dispatch-panel/dispatching";

/** The title bar's field, which the panel hangs from. */
export const ONE_BAR = ".armada-title-bar__search";

export function useOneBar({ live, dispatching }: { live: boolean; dispatching: Dispatching }): PaletteDispatch {
  useOpenedToDispatch(live, dispatching.openNew);

  return {
    kind: dispatching.kind,
    kinds: dispatching.kinds,
    ...(dispatching.kinds.length < 2 ? {} : { onKind: dispatching.choose }),
    onDispatch: (words) => dispatching.openTitled(words),
  };
}

/**
 * `n` and ⌘N open the dispatch panel, from every surface. Read in the capture phase, so the surface under it
 * never hears the key. `n` leaves a field alone; ⌘N does not. Neither acts over an open dialog, which owns the
 * keyboard. Where a panel was put away holding words, they are the draft and the panel comes back holding them.
 */
function useOpenedToDispatch(live: boolean, open: () => void): void {
  const held = useRef({ live, open });
  held.current = { live, open };
  useEffect(() => {
    function press(event: KeyboardEvent): void {
      if (event.repeat || event.defaultPrevented || !held.current.live) return;
      // `new_job` and `dispatch_from_field`, as this person has them. A capital N is a Studio's Note,
      // and the keymap reads shift exactly, so a bare press is the lowercase n.
      const bare = isPressed("new_job", event);
      const chord = !bare && isPressed("dispatch_from_field", event);
      if (!bare && !chord) return;
      if ((!chord && holdsText(event.target)) || document.querySelector('[role="dialog"]') !== null) return;
      event.preventDefault();
      event.stopPropagation();
      held.current.open();
    }
    window.addEventListener("keydown", press, true);
    return () => window.removeEventListener("keydown", press, true);
  }, []);
}
