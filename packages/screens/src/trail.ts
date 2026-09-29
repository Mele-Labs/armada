// The way back, inside one Job: where a person was before a press in one
// destination's panel took them to another's — Plan's task panel to its Drone,
// the Drone back to a task, a Check on the plan board to its Record row.
//
// **The owner's, 29 Sep 2026, and an experiment**: *"there's no way to go
// back"*. Built beside the other answer to the same note — the Drone opening
// beside the task rather than in its own destination — so the two can be
// compared.
//
// **The trail is the jump's and nobody else's.** It clears the moment a person
// moves themselves: a destination pressed on the strip, or the panel they
// landed in closed — Close and `Esc` keep meaning close, rather than turning
// into a second back. Leaving the Job clears it with everything else, since
// `JobDetail` remounts per Job.

import { useEffect, useRef, useState } from "react";
import type { SheetBack } from "@armada/components";

import { TAB_LABEL, type DetailTab } from "./detail-tabs";

/** The panel open in a destination: its id, and the words that name it — `T6`, `Drone on T6`. */
export type Open = { id: string; label: string };

/** Where a person was: the destination, and the panel open in it where one was. */
export type Place = { tab: DetailTab; open?: Open };

/**
 * What a destination that can be jumped into and out of takes. `back` is
 * handed to its sheet; `onHere` is told which panel is open, every time that
 * changes, with `null` for none.
 */
export type TrailProps = { back: SheetBack | undefined; onHere: (open: Open | null) => void };

/** The registry's `history` act, back half — `actions.toml`, `⌘[ ⌘]`. */
const BACK_KEY = "⌘[";

export function useTrail(restore: (to: Place) => void) {
  const [trail, setTrail] = useState<readonly Place[]>([]);
  // Where the person is now. **A ref, not state**: it is read only at the
  // moment of a jump, and a render per panel opened would buy nothing.
  const here = useRef<Place | undefined>(undefined);

  const last = trail[trail.length - 1];
  const back = () => {
    if (last === undefined) return;
    setTrail(trail.slice(0, -1));
    restore(last);
  };

  // ⌘[ pops while there is somewhere to go, and is nobody's otherwise.
  const backRef = useRef(back);
  backRef.current = back;
  const live = last !== undefined;
  useEffect(() => {
    if (!live) return;
    function onKey(event: KeyboardEvent) {
      if (event.defaultPrevented || !event.metaKey || event.key !== "[") return;
      event.preventDefault();
      backRef.current();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [live]);

  return {
    /** A person moved themselves. */
    clear: () => setTrail([]),
    /** A press is about to leave `from` for another destination: remember where it was. */
    push: (from: DetailTab) => {
      const at = here.current?.tab === from ? here.current : { tab: from };
      setTrail((was) => [...was, at]);
    },
    /** What the destination `tab` takes. */
    of: (tab: DetailTab): TrailProps => ({
      back:
        last === undefined
          ? undefined
          : {
              label: `Back to ${last.open?.label ?? TAB_LABEL[last.tab]}`,
              tooltip:
                last.open === undefined
                  ? `Back to ${TAB_LABEL[last.tab]}`
                  : `Back to ${TAB_LABEL[last.tab]} · ${last.open.label}`,
              binding: BACK_KEY,
              onBack: back,
            },
      onHere: (open) => {
        here.current = open === null ? { tab } : { tab, open };
        // The panel landed in has closed, which is the person moving on.
        if (open === null) setTrail([]);
      },
    }),
  };
}
