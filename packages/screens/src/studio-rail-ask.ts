// An open Studio's rail, reached from the command palette — the owner, 2 Oct 2026. Run: "Add it,
// with R as the original design drew it." Choosing the row opens the rail's Run menu, as a press on
// Run does, and the row is off for the reason Run is off. Note, Link, Sketch and Zone the same way:
// choosing one arms that kind on the rail, as its key does, and the rows are off for the rail's
// reason.
//
// **A small store of its own, because no prop path joins the two.** The palette's acts are built
// in `App.tsx`, which sits at the length the gate refuses, and whether the rail is on is worked out
// deep inside `Studios`. The rail says what it is while it is drawn, and takes the palette's ask;
// the palette reads the one and sends the other. Nothing here holds a Studio.

import { useEffect, useRef, useSyncExternalStore } from "react";
import type { StudioNodeByHandKind } from "@armada/components";

/**
 * One act on the rail, as the palette reaches it: why it is off on the rail drawn now —
 * `undefined` where it is on, `null` where no rail is drawn — and who to ask.
 */
function railAct<Ask>() {
  let off: string | undefined | null = null;
  const watching = new Set<() => void>();
  const asking = new Set<(asked: Ask) => void>();
  const said = (why: string | undefined | null) => {
    off = why;
    for (const one of watching) one();
  };
  return {
    /** The rail's half: say whether the act is on, for as long as the rail is drawn, and take the ask. */
    useRail(why: string | undefined, onAsked: (asked: Ask) => void): void {
      useEffect(() => {
        said(why);
        return () => said(null);
      }, [why]);
      const latest = useRef(onAsked);
      latest.current = onAsked;
      useEffect(() => {
        const ask = (asked: Ask) => latest.current(asked);
        asking.add(ask);
        return () => void asking.delete(ask);
      }, []);
    },
    /** The palette's half: why its row is off. */
    useOff(): string | undefined | null {
      return useSyncExternalStore(
        (changed) => {
          watching.add(changed);
          return () => void watching.delete(changed);
        },
        () => off,
      );
    },
    /** Ask the rail. Nothing where no rail is drawn. */
    ask(asked: Ask): void {
      for (const one of asking) one(asked);
    },
  };
}

const run = railAct<void>();
const add = railAct<StudioNodeByHandKind>();

/** The rail's half of Run: say whether it is on, and open the menu when the palette asks. */
export const useRailRun = run.useRail;
/** Why the palette's Run row is off, `undefined` where it is on, `null` with no Studio open. */
export const useStudioRunOff = run.useOff;
/** Open the rail's Run menu, as a press on Run would. Nothing where no rail is drawn. */
export const askStudioRun = (): void => run.ask();

/** The rail's half of Note, Link, Sketch and Zone: say whether they are on, and arm the kind asked for. */
export const useRailAdd = add.useRail;
/** Why the palette's four Add rows are off, `undefined` where they are on, `null` with no Studio open. */
export const useStudioAddOff = add.useOff;
/** Arm a kind on the rail, as its key would. Nothing where no rail is drawn. */
export const askStudioAdd = add.ask;
