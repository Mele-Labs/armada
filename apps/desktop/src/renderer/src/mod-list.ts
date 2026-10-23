// The mods on this machine, as Fleet's list holds them, for what a teaching made: a tweak laid over the theme, a Words mod that links, a
// Fleet rule, and the Mods page's rows. One held list per `window.armada`, replaced whole by every `mods.changed`.

import { useEffect, useState } from "react";
import type { ModSummary } from "@armada/protocol";

import { withState } from "./state-feed";

/** The list a window holds: empty until Fleet has answered. */
export function useModList(): readonly ModSummary[] {
  const [mods, setMods] = useState<readonly ModSummary[]>([]);
  useEffect(() => {
    const feed = withState(window.armada);
    let live = true;
    const take = (list: readonly ModSummary[] | undefined) => live && setMods((was) => (sameList(was, list ?? []) ? was : (list ?? [])));
    void feed.state().then((state) => take(state.mods?.mods));
    const off = feed.subscribe((state) => take(state.mods?.mods));
    return () => {
      live = false;
      off();
    };
  }, []);
  return mods;
}

const sameList = (a: readonly ModSummary[], b: readonly ModSummary[]): boolean => a === b || JSON.stringify(a) === JSON.stringify(b);
