// Where Bridge's tracker mods come from. The Mods page reads this one interface, so a mock and Fleet's mod
// list are interchangeable behind it. `trackerFromWire` is the reading of one `list_mods` row.

import { createContext, useContext, useSyncExternalStore } from "react";

import type { ModTried } from "@armada/components";
import type { ModSummary } from "@armada/protocol";

/**
 * A tracker mod as the Mods page draws it. **A Manifest uses it only while it is tried, valid and switched on.**
 * `tried.state` is `stale` when the file was edited after the try, which `list_mods` does not say: it leaves `tried`
 * off for a stale mod and for one never tried alike, so only a source that can tell the two draws `stale`.
 */
export type TrackerMod = {
  name: string;
  title: string;
  enabled: boolean;
  valid: boolean;
  /** Fleet's refusal, where `valid` is false. */
  problem?: string;
  /** The branch of the repository it was put on, once it has been. */
  branch?: string;
  tried: ModTried;
};

export interface TrackerSource {
  get(): readonly TrackerMod[];
  subscribe(on: () => void): () => void;
  setEnabled(name: string, enabled: boolean): void;
  promote(name: string): void;
  /** Try the mod against the repository, which keeps how it went. */
  tryIt(name: string): void;
}

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const dayOf = (at: string) => {
  const day = new Date(at);
  return Number.isNaN(day.getTime()) ? at : `${day.getUTCDate()} ${MONTHS[day.getUTCMonth()]}`;
};

/** One tracker mod off `list_mods`, or `undefined` for any other kind. `stale` is the caller's, for the reason on `TrackerMod`. */
export function trackerFromWire(mod: ModSummary, stale = false): TrackerMod | undefined {
  if (mod.kind !== "tracker") return undefined;
  const tried: ModTried =
    mod.tried === undefined
      ? stale
        ? { state: "stale", words: "Stale" }
        : { state: "never", words: "Not tried" }
      : mod.tried.passed
        ? { state: "passed", words: `Tried ${dayOf(mod.tried.at)} on ${mod.tried.manifest_id}` }
        : { state: "failed", words: `Failed ${dayOf(mod.tried.at)}: ${mod.tried.reason ?? "no reason given"}` };
  return { name: mod.name, title: mod.name, enabled: mod.enabled, valid: mod.valid, ...(mod.reason === undefined ? {} : { problem: mod.reason }), tried };
}

/** A source held in memory: the mock runs on it. */
export function createTrackerSource(initial: () => readonly TrackerMod[] = () => []): TrackerSource & {
  /** A mod arriving, as one written into the mod folder would. */
  install(mod: TrackerMod): void;
  reset(): void;
} {
  let held = initial();
  const listeners = new Set<() => void>();
  const set = (next: readonly TrackerMod[]) => {
    held = next;
    listeners.forEach((on) => on());
  };
  const edit = (name: string, change: Partial<TrackerMod>) => set(held.map((mod) => (mod.name === name ? { ...mod, ...change } : mod)));
  return {
    get: () => held,
    subscribe: (on) => {
      listeners.add(on);
      return () => void listeners.delete(on);
    },
    setEnabled: (name, enabled) => edit(name, { enabled }),
    promote: (name) => edit(name, { branch: `armada/mod-${name}-${Date.now()}` }),
    tryIt: (name) => edit(name, { tried: { state: "passed", words: "Tried today" } }),
    install: (mod) => held.some((one) => one.name === mod.name) || set([...held, mod]),
    reset: () => set(initial()),
  };
}

const LOCAL = createTrackerSource();
const TrackerSourceContext = createContext<TrackerSource>(LOCAL);

export const TrackerSourceProvider = TrackerSourceContext.Provider;

/** The tracker mods in force, and the source that serves them. */
export function useTrackers(): { mods: readonly TrackerMod[]; source: TrackerSource } {
  const source = useContext(TrackerSourceContext);
  const mods = useSyncExternalStore(source.subscribe, source.get);
  return { mods, source };
}
