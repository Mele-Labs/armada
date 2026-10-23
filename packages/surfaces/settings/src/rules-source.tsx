// Where Bridge's rules mods come from. A rules mod keeps what a teaching changed in Bridge and Fleet, one per teaching,
// named from it. The Rules list reads this one interface, as the Theme and Layout lists read theirs, so a mock and Fleet's
// mod folder are interchangeable behind it. Nothing here applies a rule: the rule's own reader checks the switch.

import { createContext, useContext, useSyncExternalStore } from "react";

/** One thing a rule says: a pattern, an address, a notice, what Fleet does. `from` is where it was found, absent where it was said. */
export type RuleFact = { label: string; value: string; code?: boolean; from?: string };

/** A rules mod on this machine. */
export type RulesMod = {
  /** The folder's name, a slug, and what the row is called. */
  name: string;
  /** What it does, in one line. */
  description: string;
  /** This machine's switch for the mod. */
  enabled: boolean;
  /** The branch of the repository it was put on, once it has been. */
  branch?: string;
  /** What it holds, a part for each of Bridge and Fleet. */
  rules: readonly { part: "Bridge" | "Fleet"; facts: readonly RuleFact[] }[];
};

export type RulesState = {
  mods: readonly RulesMod[];
  /** The mod asked for by name, shown open. */
  opened?: string;
};

export interface RulesSource {
  get(): RulesState;
  subscribe(on: () => void): () => void;
  setEnabled(name: string, enabled: boolean): void;
  promote(name: string): void;
  /** Show one mod open, or none. */
  open(name: string | undefined): void;
}

const NONE: RulesState = { mods: [] };
const LOCAL: RulesSource = { get: () => NONE, subscribe: () => () => undefined, setEnabled: () => undefined, promote: () => undefined, open: () => undefined };
const RulesSourceContext = createContext<RulesSource>(LOCAL);

export const RulesSourceProvider = RulesSourceContext.Provider;

/** The source in force, and its state as it stands. A window with no source has no rules mod. */
export function useRules(): RulesState & { source: RulesSource } {
  const source = useContext(RulesSourceContext);
  const state = useSyncExternalStore(source.subscribe, source.get, source.get);
  return { ...state, source };
}
