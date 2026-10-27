// The title bar's search field and the keys that dispatch. `n` and ⌘N open the dispatch sheet on the draft, in one
// press, from every surface; the palette is search only, and a dispatch row chosen in it (↵ on the row, ⌘↵, a pasted
// link) moves the words into that same sheet, in the kind Tab chose. The sheet, its draft and the sends are `dispatch-panel/`.
// Task and Project, where the window keeps projects, file the words in them instead (`projects-tab/palette.ts`).

import { useEffect, useRef, useState } from "react";
import { adds, isPressed, type AddKind, type BarKind, type PaletteDispatch } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import { holdsText } from "@armada/screens/src/keys";

import type { Dispatching } from "./dispatch-panel/dispatching";
import { useProjectsFiling } from "./projects-tab/palette";

/** The title bar's field, which the strip a send leaves hangs from. */
export const ONE_BAR = ".armada-title-bar__search";

export function useOneBar({ live, dispatching, open, picked, repositories }: {
  live: boolean;
  dispatching: Dispatching;
  /** Whether the palette is up: closed, Tab's Task or Project goes back to the sheet's kind. */
  open: boolean;
  picked: RepositorySummary | null;
  repositories: readonly RepositorySummary[];
}): PaletteDispatch {
  useOpenedToDispatch(live, dispatching.openNew, dispatching.openAfter);
  const filing = useProjectsFiling(picked, repositories);
  const [adding, setAdding] = useState<AddKind>();
  useEffect(() => {
    if (!open) setAdding(undefined);
  }, [open]);
  const kinds: BarKind[] = [...dispatching.kinds, ...(filing === undefined ? [] : (["task", "project"] as const))];

  return {
    kind: adding ?? dispatching.kind,
    kinds,
    // Tab past the sheet's kinds puts the sheet back on its first, so the palette opens on Job again once Task or Project was chosen.
    ...(kinds.length < 2
      ? {}
      : { onKind: (next: BarKind) => (adds(next) ? (setAdding(next), dispatching.choose(dispatching.kinds[0] ?? "session")) : (setAdding(undefined), dispatching.choose(next))) }),
    onDispatch: (words, kind) => (adds(kind) ? filing?.add(words, kind) : dispatching.openSheet(words)),
    ...(filing === undefined ? {} : { step: (words: string, kind: BarKind) => (adds(kind) ? filing.step(words, kind) : undefined) }),
  };
}

/**
 * `n` and ⌘N open the dispatch sheet, from every surface; ⌥A opens it on a Session with an `@` typed. Read in the capture phase, so the surface under it never
 * hears the key. `n` leaves a field alone; ⌘N does not. Neither acts over an open dialog, which owns the keyboard.
 * Where a sheet was put away holding words, they are the draft and the sheet comes back holding them.
 */
function useOpenedToDispatch(live: boolean, open: () => void, openAfter: (() => void) | undefined): void {
  const held = useRef({ live, open, openAfter });
  held.current = { live, open, openAfter };
  useEffect(() => {
    function press(event: KeyboardEvent): void {
      if (event.repeat || event.defaultPrevented || !held.current.live) return;
      // `dispatch_after`: a Session that waits on another. Option changes the character A types, so the registry's reader takes the key by its place.
      if (isPressed("dispatch_after", event) && held.current.openAfter !== undefined) {
        if (holdsText(event.target) || document.querySelector('[role="dialog"]') !== null) return;
        event.preventDefault();
        event.stopPropagation();
        held.current.openAfter();
        return;
      }
      // `new_job` and `dispatch_from_field`, as this person has them. A capital N is a Studio's Note,
      // and the keymap reads shift exactly, so a bare press is the lowercase n.
      const bare = isPressed("new_job", event);
      const chord = !bare && isPressed("dispatch_from_field", event);
      if (!bare && !chord) return;
      // A row that marks `n` its own, such as a drift's Not a drift on the Workflows page, keeps the key.
      if (bare && event.target instanceof Element && event.target.closest("[data-own-keys~='n']") !== null) return;
      if ((!chord && holdsText(event.target)) || document.querySelector('[role="dialog"]') !== null) return;
      event.preventDefault();
      event.stopPropagation();
      held.current.open();
    }
    window.addEventListener("keydown", press, true);
    return () => window.removeEventListener("keydown", press, true);
  }, []);
}
