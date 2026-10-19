// The window's own effects, out of `App.tsx`, which is at the length the gate refuses: where a pressed notification lands and the
// call an alert asked the Cockpit for. Each is App's, and each says why it is the way it is.

import { useEffect, useRef, useState } from "react";
import type { Summons } from "../../shared/bridge";
import { askToOpenSession } from "./open-session";

/**
 * **Where a pressed notification says to go, and it always goes somewhere.** A press that raised
 * the window and left it on whatever it was last showing is a press that did nothing, which is
 * the one outcome that teaches somebody to stop pressing them.
 *
 * A Session opens on the Cockpit. Anything else lands on the Cockpit itself, the overlays down
 * first: the press asked for the work, not for the composer that happened to be up.
 */
export function useSummoned(go: () => void, then: (act: NonNullable<Summons["then"]>, sessionId: string) => void): void {
  // The latest, because the listener is installed once.
  const latest = useRef({ go, then });
  latest.current = { go, then };
  useEffect(
    () =>
      window.armada.onSummoned((to) => {
        if (to.sessionId === undefined) return latest.current.go();
        // A walk window's panel asked for more than the Session: a Check or its Loadout opens over it, a surface or a link replaces it.
        if (to.then === undefined || to.then.kind === "check" || to.then.kind === "loadout") askToOpenSession(to.sessionId);
        if (to.then !== undefined) latest.current.then(to.then, to.sessionId);
      }),
    [],
  );
}

/**
 * Where a press asked the Cockpit to land: one call (an in-app alert's). **A token rather than a call**: the press may have
 * arrived over another page, so the Cockpit is not mounted yet, and it acts once it is. `at` is what makes a second press of the same
 * call land, and the taker clears the token so it is not acted on twice.
 */
export function useLandings() {
  const [bringing, setBringing] = useState<{ key: string; at: number } | null>(null);
  return {
    bringing,
    bring: (key: string) => setBringing({ key, at: Date.now() }),
    brought: () => setBringing(null),
  };
}
