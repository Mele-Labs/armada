// Write clashes, as the window holds them. Fleet detects two live agents in one repository (a Job's
// Drone, a Session) whose changes really conflict, or one agent against a main that moved, and
// tells both. The shapes below mirror what `GET /clashes?manifest_id=&holder=` returns. Mock only
// for now: a scenario carries the list, nothing reads the wire, and the app's own mount provides none.

import { useMemo } from "react";
import type { ReactNode } from "react";
import { ClashesProvider } from "@armada/components";
import type { ClashHolderRef, Clashing } from "@armada/components";

import { askToOpenHolder } from "./open-holder";

export type ClashHolder = ClashHolderRef;

export type WriteClash = {
  one: ClashHolder;
  other: ClashHolder;
  paths: string[];
  /** When Fleet first saw the conflict. */
  since: string;
};

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

/** Hands the window what Fleet said clashes. Nothing handed, nothing is marked. */
export function ClashesFrom({ clashes, children }: { clashes: readonly WriteClash[] | undefined; children: ReactNode }) {
  const value = useMemo<Clashing>(() => ({ of: clashingOf(clashes ?? []), open: askToOpenHolder }), [clashes]);
  return clashes === undefined ? <>{children}</> : <ClashesProvider value={value}>{children}</ClashesProvider>;
}
