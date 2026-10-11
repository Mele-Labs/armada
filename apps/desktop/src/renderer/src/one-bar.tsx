// The title bar's one bar: search and dispatch in one field, proposal D (the owner, 10 Oct 2026). It
// replaced the Dashboard's dispatch bar and the title bar's Dispatch button. The palette is the field,
// and this is what it sends: a Job goes straight to the proposer where one repository is picked, and
// to the composer on All, which asks which. A Session starts as the dispatch bar started one.

import { useEffect, useRef, useState } from "react";
import { isPressed, type DispatchKind, type PaletteDispatch } from "@armada/components";
import { holdsText } from "@armada/screens/src/keys";

import { useSessionsDraft } from "./sessions-draft";
import { useDispatchKind } from "./remembered-views";

/** The title bar's field, which the palette opens over. */
export const ONE_BAR = ".armada-title-bar__search";

export function useOneBar({
  palette,
  live,
  repository,
  compose,
  openSession,
  said,
  proposeFrom,
}: {
  palette: { open: boolean; onOpen: () => void };
  /** Off the line, `n` keeps the Board's own meaning, as it did beside the dispatch bar. */
  live: boolean;
  /** The picked repository, or `null` on All. */
  repository: string | null;
  /** The full composer, holding the words. */
  compose: (words: string) => void;
  openSession: (sessionId: string) => void;
  said: (sentence: string) => void;
  proposeFrom: (request: string, attachments: readonly [], repository: null) => Promise<string | null>;
}): PaletteDispatch {
  const draft = useSessionsDraft();
  const [chosen, choose] = useDispatchKind();
  const kind: DispatchKind = draft === undefined ? "job" : chosen;
  const [leads, setLeads] = useOpenedToDispatch(palette, live);

  useEffect(() => void (palette.open || setLeads(false)), [palette.open, setLeads]);

  return {
    kind,
    ...(draft === undefined ? {} : { onKind: choose }),
    leads,
    onCompose: compose,
    onDispatch: (words, as) => {
      if (as === "session" && draft !== undefined) {
        void Promise.resolve(draft.start()).then((id) => {
          if (id === undefined) return;
          draft.send(id, { text: words, files: [], sketches: [], tags: [] });
          openSession(id);
        });
      } else if (repository === null) compose(words);
      else void proposeFrom(words, [], null).then((told) => told === null || said(told));
    },
  };
}

/**
 * `n` and ⌘N open the one bar with its dispatch row leading, from every surface. Read in the capture
 * phase, so the surface under it never hears the key. `n` leaves a field alone; ⌘N does not. Neither
 * acts over an open dialog, which owns the keyboard.
 */
function useOpenedToDispatch(
  palette: { open: boolean; onOpen: () => void },
  live: boolean,
): [boolean, (leads: boolean) => void] {
  const [leads, setLeads] = useState(false);
  const held = useRef({ palette, live });
  held.current = { palette, live };
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
      setLeads(true);
      held.current.palette.onOpen();
    }
    window.addEventListener("keydown", press, true);
    return () => window.removeEventListener("keydown", press, true);
  }, []);
  return [leads, setLeads];
}
