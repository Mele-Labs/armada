// The pull requests, issues, tasks and commands the palette lists. **A mock scenario hands its own** (`palette` on its
// draft), and they stand in for the real ones so a walk draws the same screen every time. Without any, the window reads
// them: pull requests and issues from Fleet when the palette opens (`palette-tickets.ts`). His tasks are the Projects tab's (`projects-tab/palette.ts`), handed to the mount. The
// commands a Commands mod brings have no Fleet source yet, so a real window lists none.

import { createContext, useContext, useEffect, useState, useSyncExternalStore, type ReactNode } from "react";
import type { PaletteCommand, PaletteTicket } from "@armada/shell";

import type { BridgeApi } from "../../shared/api";
import { PaletteTickets } from "./palette-tickets";
import { withState } from "./state-feed";

/** A ticket with the address its page opens from. */
export type PaletteLinked = PaletteTicket & { url: string };

export type PaletteFixtures = {
  pulls: readonly PaletteLinked[];
  issues: readonly PaletteLinked[];
  commands: readonly PaletteCommand[];
  /** Row ids, as the palette draws them, pinned and opened before the window opened. */
  pinned?: readonly string[];
  recent?: readonly string[];
};

const NONE: PaletteFixtures = { pulls: [], issues: [], commands: [] };
const Source = createContext<PaletteFixtures>(NONE);

/** Hands the window the rows. Nothing handed, none are drawn. */
export function PaletteFixturesFrom({ fixtures, children }: { fixtures: PaletteFixtures | undefined; children: ReactNode }) {
  return fixtures === undefined ? <>{children}</> : <Source.Provider value={fixtures}>{children}</Source.Provider>;
}

export const usePaletteFixtures = (): PaletteFixtures => useContext(Source);

const NEVER = (): (() => void) => () => undefined;
const NO_TICKETS = { pulls: [], issues: [] } as const;

/**
 * What the palette lists: a scenario's rows where it handed any, otherwise Fleet's. Reading starts when the palette opens, and
 * it holds what it read from then on, so the next opening draws at once and refreshes behind it.
 */
export function usePaletteRows(open: boolean): PaletteFixtures {
  const given = usePaletteFixtures();
  const [store] = useState(() => {
    const api = window.armada as Partial<BridgeApi> | undefined;
    // A test's fake window with no state to follow.
    return typeof api?.syncState === "function" && typeof api.subscribeChanges === "function" ? new PaletteTickets(withState(api as BridgeApi)) : undefined;
  });
  useEffect(() => store?.attach(), [store]);
  useEffect(() => {
    if (open) store?.load();
  }, [open, store]);
  const tickets = useSyncExternalStore(store?.subscribe ?? NEVER, store?.get ?? (() => NO_TICKETS));
  return {
    ...given,
    pulls: given.pulls.length > 0 ? given.pulls : tickets.pulls,
    issues: given.issues.length > 0 ? given.issues : tickets.issues,
  };
}
