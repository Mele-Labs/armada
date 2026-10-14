// The title bar's one bar: search and dispatch in one field, proposal D (the owner, 10 Oct 2026). It
// replaced the Dashboard's dispatch bar and the title bar's Dispatch button; the Cockpit has no bar of its own now. The palette is the field,
// and a dispatch row chosen in it (↵ on the row, ⌘↵, a pasted link) does not send: it grows the dispatch
// panel down from the field, holding the words, in the kind Tab chose. The panel, its draft and the
// sends are `dispatch-panel/`.

import { useEffect, useRef, useState } from "react";
import { isPressed, type PaletteDispatch } from "@armada/components";
import { holdsText } from "@armada/screens/src/keys";

import type { Dispatching } from "./dispatch-panel/dispatching";

/** The title bar's field, which the palette opens over. */
export const ONE_BAR = ".armada-title-bar__search";

export function useOneBar({
  palette,
  live,
  compose,
  dispatching,
}: {
  palette: { open: boolean; onOpen: () => void };
  /** Off the line, `n` keeps the Board's own meaning, as it did before the one bar. */
  live: boolean;
  /** The full composer, holding the words. */
  compose: (words: string) => void;
  dispatching: Dispatching;
}): PaletteDispatch {
  const [leads, setLeads] = useOpenedToDispatch(palette, live, dispatching.reopenDraft);

  useEffect(() => void (palette.open || setLeads(false)), [palette.open, setLeads]);

  return {
    kind: dispatching.kind,
    kinds: dispatching.kinds,
    ...(dispatching.kinds.length < 2 ? {} : { onKind: dispatching.choose }),
    leads,
    onCompose: compose,
    onDispatch: (words) => dispatching.openTitled(words),
  };
}

/**
 * `n` and ⌘N open the one bar with its dispatch row leading, from every surface. Read in the capture
 * phase, so the surface under it never hears the key. `n` leaves a field alone; ⌘N does not. Neither
 * acts over an open dialog, which owns the keyboard. Where a panel was put away holding words, they are the
 * draft and the key brings that panel back instead.
 */
function useOpenedToDispatch(
  palette: { open: boolean; onOpen: () => void },
  live: boolean,
  reopenDraft: () => boolean,
): [boolean, (leads: boolean) => void] {
  const [leads, setLeads] = useState(false);
  const held = useRef({ palette, live, reopenDraft });
  held.current = { palette, live, reopenDraft };
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
      // Words left in a panel put away are the draft: the key brings the panel back holding them.
      if (held.current.reopenDraft()) return;
      setLeads(true);
      held.current.palette.onOpen();
    }
    window.addEventListener("keydown", press, true);
    return () => window.removeEventListener("keydown", press, true);
  }, []);
  return [leads, setLeads];
}
