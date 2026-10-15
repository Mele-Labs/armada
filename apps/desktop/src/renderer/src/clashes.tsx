// Write clashes, as the window holds them. Fleet detects two live agents in one repository (a Job's
// Drone, a Session) whose changes really conflict, or one agent against a main that moved, and tells
// both; `GET /clashes` lists what stands (`@armada/protocol`, `WriteClash`). `fleet-clashes.ts` reads
// it in the app and a mock scenario carries its own list; both hand the list to `ClashesFrom`.

import { createContext, useContext, useMemo, useSyncExternalStore } from "react";
import type { ReactNode } from "react";
import { ClashesProvider } from "@armada/components";
import type { Clashing } from "@armada/components";
import type { WriteClash } from "@armada/protocol";

import type { ClashesSource } from "./fleet-clashes";
import { askToOpenHolder } from "./open-holder";

/** What each holder sees: the clashes it is in, each as the other holder and the paths. */
export function clashingOf(clashes: readonly WriteClash[]): Clashing["of"] {
  return (kind, id) =>
    clashes.flatMap((clash) =>
      clash.one.kind === kind && clash.one.id === id
        ? [{ with: clash.other, paths: clash.paths }]
        : clash.other.kind === kind && clash.other.id === id
          ? [{ with: clash.one, paths: clash.paths }]
          : [],
    );
}

const NONE: readonly WriteClash[] = [];
const Written = createContext<readonly WriteClash[]>(NONE);

/** The list as Fleet gave it, for a reader that draws the clashes themselves and not what each holder is in: the flight deck's map. */
export const useWriteClashes = (): readonly WriteClash[] => useContext(Written);

/** Hands the window what Fleet said clashes. Nothing handed, nothing is marked. */
export function ClashesFrom({ clashes, children }: { clashes: readonly WriteClash[] | undefined; children: ReactNode }) {
  const value = useMemo<Clashing>(() => ({ of: clashingOf(clashes ?? []), open: askToOpenHolder }), [clashes]);
  return clashes === undefined ? <>{children}</> : <Written.Provider value={clashes}><ClashesProvider value={value}>{children}</ClashesProvider></Written.Provider>;
}

/** The clashes Fleet reports, kept current by `source`. The provider is always there, so a read landing does not remount the app. */
export function ClashesFromFleet({ source, children }: { source: ClashesSource; children: ReactNode }) {
  const clashes = useSyncExternalStore(source.subscribe, source.get);
  return <ClashesFrom clashes={clashes}>{children}</ClashesFrom>;
}
