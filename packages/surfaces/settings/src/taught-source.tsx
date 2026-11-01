// Where Bridge's taught mods come from. A teaching changes how Bridge shows words, what Fleet does with a notice, or a shared colour
// of the theme, and each change is a mod of its own kind: Words, Fleet or a Theme tweak. A change to the layout is a layout mod, which
// the layout source holds, so this one only describes it. The lists read this one interface, as the Theme and Layout lists read
// theirs, so a mock and Fleet's mod folder are interchangeable behind it. Nothing here applies a change: its own reader checks the switch.

import { createContext, useContext, useSyncExternalStore } from "react";

import type { ModTracker } from "@armada/components";
import type { ModSummary } from "@armada/protocol";

/** One thing a mod says: a pattern, an address, a notice, a colour. `from` is where it was found, absent where it was said. */
export type TaughtFact = { label: string; value: string; code?: boolean; from?: string };

export type TaughtPart = { part: string; facts: readonly TaughtFact[] };

/** What a mod changes besides the theme and the layout: how Bridge shows words, what Fleet does with a notice, a shared colour over the theme, an issue tracker Fleet was not built for. */
export type TaughtKind = "words" | "fleet" | "tweak" | "tracker" | "tab";

/** A mod of one of those kinds on this machine. */
export type TaughtMod = {
  /** The folder's name, a slug, and what the row is called. */
  name: string;
  kind: TaughtKind;
  /** What it does, in one line. */
  description: string;
  /** What it was taught on, as a row says it. */
  on: string;
  /** This machine's switch for the mod. */
  enabled: boolean;
  /** The branch of the repository it was put on, once it has been. */
  branch?: string;
  /** What it holds. */
  parts: readonly TaughtPart[];
  /** A tracker mod's tried and valid marks. Absent for any other kind. */
  tracker?: ModTracker;
};

/** What a mod was taught on and what it holds, for any kind, a layout mod's included. */
export type TaughtDetail = { on: string; parts: readonly TaughtPart[] };

export type TaughtState = {
  mods: readonly TaughtMod[];
  /** By mod name. */
  details: Readonly<Record<string, TaughtDetail>>;
  /** The mod asked for by name, shown open. */
  opened?: string;
};

export interface TaughtSource {
  get(): TaughtState;
  subscribe(on: () => void): () => void;
  setEnabled(name: string, enabled: boolean): void;
  promote(name: string): void;
  /** Show one mod open, or none. */
  open(name: string | undefined): void;
  /** Try a tracker mod against the repository. Absent where Bridge cannot ask Fleet to. */
  tryIt?(name: string): void;
}

const NONE: TaughtState = { mods: [], details: {} };
const LOCAL: TaughtSource = { get: () => NONE, subscribe: () => () => undefined, setEnabled: () => undefined, promote: () => undefined, open: () => undefined };
const TaughtSourceContext = createContext<TaughtSource>(LOCAL);

export const TaughtSourceProvider = TaughtSourceContext.Provider;

/** The source in force, and its state as it stands. A window with no source has no taught mod. */
export function useTaught(): TaughtState & { source: TaughtSource } {
  const source = useContext(TaughtSourceContext);
  const state = useSyncExternalStore(source.subscribe, source.get, source.get);
  return { ...state, source };
}

/** The taught mods of one kind. */
export const taughtOf = (state: TaughtState, kind: TaughtKind): readonly TaughtMod[] => state.mods.filter((one) => one.kind === kind);

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const dayOf = (at: string) => {
  const day = new Date(at);
  return Number.isNaN(day.getTime()) ? at : `${day.getUTCDate()} ${MONTHS[day.getUTCMonth()]}`;
};

/**
 * A tracker mod's marks off its `list_mods` row. **A row with no `tried` is `stale` where Fleet says the `tracker.toml` was edited after
 * its try**, and `never` otherwise.
 */
export function trackerOf(row: ModSummary): ModTracker {
  const tried = row.tried;
  return {
    valid: row.valid,
    tried:
      tried === undefined
        ? row.stale === true
          ? { state: "stale", words: "Stale" }
          : { state: "never", words: "Not tried" }
        : tried.passed
          ? { state: "passed", words: `Tried ${dayOf(tried.at)} on ${tried.manifest_id}` }
          : { state: "failed", words: `Failed ${dayOf(tried.at)}: ${tried.reason ?? "no reason given"}` },
  };
}
