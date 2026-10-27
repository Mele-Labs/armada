// The pull requests, issues, tasks and commands the palette lists. **Fleet serves none of them to it yet**, so the
// window's source is empty and a scenario hands stand-ins; the wiring step reads the real ones.

import { createContext, useContext, type ReactNode } from "react";
import type { PaletteCommand, PaletteTask, PaletteTicket } from "@armada/shell";

/** A ticket with the address its page opens from. */
export type PaletteLinked = PaletteTicket & { url: string };

export type PaletteFixtures = {
  pulls: readonly PaletteLinked[];
  issues: readonly PaletteLinked[];
  tasks: readonly PaletteTask[];
  commands: readonly PaletteCommand[];
  /** Row ids, as the palette draws them, pinned and opened before the window opened. */
  pinned?: readonly string[];
  recent?: readonly string[];
};

const NONE: PaletteFixtures = { pulls: [], issues: [], tasks: [], commands: [] };
const Source = createContext<PaletteFixtures>(NONE);

/** Hands the window the rows. Nothing handed, none are drawn. */
export function PaletteFixturesFrom({ fixtures, children }: { fixtures: PaletteFixtures | undefined; children: ReactNode }) {
  return fixtures === undefined ? <>{children}</> : <Source.Provider value={fixtures}>{children}</Source.Provider>;
}

export const usePaletteFixtures = (): PaletteFixtures => useContext(Source);
