// Run on an open Studio's rail, reached from the command palette — the owner, 2 Oct 2026: "Add it,
// with R as the original design drew it." Choosing the row opens the rail's Run menu, as a press on
// Run does, and the row is off for the reason Run is off.
//
// **A small store of its own, because no prop path joins the two.** The palette's acts are built
// in `App.tsx`, which sits at the length the gate refuses, and whether Run is on is worked out deep
// inside `Studios`. The rail says what it is while it is drawn, and takes the palette's ask; the
// palette reads the one and sends the other. Nothing here holds a Studio.

import { useEffect, useRef, useSyncExternalStore } from "react";

/** Why Run is off on the rail drawn now; `undefined` where it is on, `null` where no rail is drawn. */
let off: string | undefined | null = null;
const watching = new Set<() => void>();
const asking = new Set<() => void>();

function said(why: string | undefined | null): void {
  off = why;
  for (const one of watching) one();
}

/**
 * The rail's half: say whether Run is on, for as long as the rail is drawn,
 * and open the menu when the palette asks.
 */
export function useRailRun(why: string | undefined, onAsked: () => void): void {
  useEffect(() => {
    said(why);
    return () => said(null);
  }, [why]);
  const latest = useRef(onAsked);
  latest.current = onAsked;
  useEffect(() => {
    const ask = () => latest.current();
    asking.add(ask);
    return () => void asking.delete(ask);
  }, []);
}

/** The palette's half: why its Run row is off, `undefined` where it is on, `null` with no Studio open. */
export function useStudioRunOff(): string | undefined | null {
  return useSyncExternalStore(
    (changed) => {
      watching.add(changed);
      return () => void watching.delete(changed);
    },
    () => off,
  );
}

/** Open the rail's Run menu, as a press on Run would. Nothing where no rail is drawn. */
export function askStudioRun(): void {
  for (const ask of asking) ask();
}
