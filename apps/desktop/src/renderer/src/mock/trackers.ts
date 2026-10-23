// Mock-only: the `mods-trackers` scenario's machine. Its tracker mods are `list_mods` rows in the scenario's state, read as Bridge reads
// them. `list_mods` leaves `tried` off a mod whose `tracker.toml` was edited after its try, exactly as it does for a mod never tried, and
// Bridge has no way to ask Fleet to try one, so `triable` is the mock saying which of the two it is and answering Try itself.
// Installed once the window has reset the sources, which would otherwise clear them.

import type { TaughtMod, TaughtSource, TaughtState } from "@armada/settings";
import { parseLayout } from "@armada/shell";

import { mockLayout, TIDY_DASHBOARD } from "./layout";
import { DUSK_MOD, mockThemes } from "./themes";

/** Valid and on, edited since its try. */
export const STALE = new Set(["shortcut"]);

/** A theme in force and a layout switched off, so the list has all three of Theme, Layout and Tracker. */
export function installTrackerMods(): void {
  mockThemes.install(DUSK_MOD);
  mockThemes.setActive(DUSK_MOD.name);
  mockLayout.install({ name: "tidy", title: "Tidy", enabled: false, file: parseLayout(TIDY_DASHBOARD).file });
}

/** The same source with `STALE` drawn stale and Try answered: a tracker tried reads passed. */
export function triable(source: TaughtSource): TaughtSource {
  const tried = new Set<string>();
  const listeners = new Set<() => void>();
  let from: TaughtState | undefined;
  let version = 0;
  let seen = -1;
  let shown: TaughtState = source.get();
  const mark = (mod: TaughtMod): TaughtMod => {
    if (mod.tracker === undefined || mod.tracker.tried.state !== "never") return mod;
    if (tried.has(mod.name)) return { ...mod, tracker: { ...mod.tracker, tried: { state: "passed", words: "Tried today" } } };
    return STALE.has(mod.name) ? { ...mod, tracker: { ...mod.tracker, tried: { state: "stale", words: "Stale" } } } : mod;
  };
  return {
    ...source,
    get: () => {
      const state = source.get();
      if (state !== from || version !== seen) {
        from = state;
        seen = version;
        shown = { ...state, mods: state.mods.map(mark) };
      }
      return shown;
    },
    subscribe: (on) => {
      listeners.add(on);
      const off = source.subscribe(on);
      return () => {
        listeners.delete(on);
        off();
      };
    },
    tryIt: (name) => {
      tried.add(name);
      version += 1;
      listeners.forEach((on) => on());
    },
  };
}
