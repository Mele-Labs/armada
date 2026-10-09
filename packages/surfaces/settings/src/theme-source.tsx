// Where Bridge's themes come from. The Settings Theme card, the Mods surface and the renderer's
// loader all read this one interface, so a mock, a local default and Fleet's mod folder are
// interchangeable behind it. Nothing here touches the document: applying a theme is the renderer's.

import { createContext, useContext, useSyncExternalStore } from "react";

/** Built in, drawn by `light.css` or by the tokens' own defaults. Anything else is a mod's name. */
export const BUILT_IN_THEMES = ["dark", "light"] as const;
const DARK = "dark";

/**
 * A theme a mod ships. **Its CSS is read when the theme is drawn, not held**: `load` resolves to the
 * stylesheet Fleet checked and rejects where it did not pass, so what the loader adopts is the text
 * that was validated and never a copy kept from before.
 */
export type ThemeMod = {
  name: string;
  title: string;
  /** This machine's switch for the mod. */
  enabled: boolean;
  /** Why Bridge may not draw it, as Fleet said it. Absent while it can. */
  problem?: string;
  /** The branch of the repository it was put on, once it has been. */
  branch?: string;
  load(): Promise<string>;
};

/**
 * A theme Bridge ships. **The same shape as a mod's, delivered differently**: its CSS is one
 * `[data-theme="<id>"]` block like a mod's `theme.css`, fetched when it is chosen so that none of
 * it is in the initial stylesheet. Its id carries a `catalogue:` prefix, which no mod's name has.
 */
export type CatalogueTheme = { id: string; title: string; tone: "dark" | "light"; load(): Promise<string> };

export type ThemeState = { mods: readonly ThemeMod[]; catalogue: readonly CatalogueTheme[]; active: string };

export interface ThemeSource {
  get(): ThemeState;
  subscribe(on: () => void): () => void;
  /** Choose a theme by id. An id that is neither built in, in the catalogue nor an enabled mod becomes Dark. */
  setActive(id: string): void;
  /**
   * The loader could not draw `id`, and the window shows Dark. A source that keeps the choice
   * somewhere durable keeps it here too, since a mod that failed a check a moment ago may pass the
   * next one; without this the choice is `setActive(Dark)`, as for a source with nowhere to keep it.
   */
  fellBack?(id: string): void;
}

/** What the Mods surface adds: switching a mod on and off, and promoting it to a branch. */
export interface ModsSource extends ThemeSource {
  setEnabled(name: string, enabled: boolean): void;
  promote(name: string): void;
}

const isBuiltIn = (id: string) => (BUILT_IN_THEMES as readonly string[]).includes(id);
/** A mod Bridge may draw: switched on, and nothing wrong with it. */
export const offered = (mod: ThemeMod) => mod.enabled && mod.problem === undefined;
const usable = (state: ThemeState, id: string) => isBuiltIn(id) || state.catalogue.some((one) => one.id === id) || state.mods.some((mod) => mod.name === id && offered(mod));

/**
 * A source held in memory: the mock runs on it. Bridge runs on `fleet-themes.ts`, which answers the
 * same interface from Fleet's mod list and the saved theme preference.
 */
export function createThemeSource(initial: () => ThemeState = () => ({ mods: [], catalogue: [], active: DARK })): ModsSource & {
  /** A mod arriving, the way a Session writing one into the mod folder would. */
  install(mod: ThemeMod): void;
  reset(): void;
} {
  let held = initial();
  const listeners = new Set<() => void>();
  const set = (next: ThemeState) => {
    held = usable(next, next.active) ? next : { ...next, active: DARK };
    listeners.forEach((on) => on());
  };
  const edit = (name: string, change: Partial<ThemeMod>) => set({ ...held, mods: held.mods.map((mod) => (mod.name === name ? { ...mod, ...change } : mod)) });
  return {
    get: () => held,
    subscribe: (on) => {
      listeners.add(on);
      return () => void listeners.delete(on);
    },
    setActive: (id) => set({ ...held, active: id }),
    setEnabled: (name, enabled) => edit(name, { enabled }),
    promote: (name) => edit(name, { branch: `armada/mod-${name}-${Date.now()}` }),
    install: (mod) => held.mods.some((one) => one.name === mod.name) || set({ ...held, mods: [...held.mods, mod] }),
    reset: () => set(initial()),
  };
}

/** The same source with every mod hidden: safe mode. Built-in and catalogue themes still choose. */
export function withoutMods(source: ModsSource): ModsSource {
  let from: ThemeState | undefined;
  let shown: ThemeState = { mods: [], catalogue: [], active: DARK };
  return {
    get: () => {
      const state = source.get();
      if (state !== from) {
        from = state;
        const kept = isBuiltIn(state.active) || state.catalogue.some((one) => one.id === state.active);
        shown = { mods: [], catalogue: state.catalogue, active: kept ? state.active : DARK };
      }
      return shown;
    },
    subscribe: (on) => source.subscribe(on),
    setActive: (id) => {
      const state = source.get();
      if (isBuiltIn(id) || state.catalogue.some((one) => one.id === id)) source.setActive(id);
    },
    ...(source.fellBack === undefined ? {} : { fellBack: (id: string) => source.fellBack?.(id) }),
    setEnabled: () => undefined,
    promote: () => undefined,
  };
}

const LOCAL = createThemeSource();
const ThemeSourceContext = createContext<ModsSource>(LOCAL);

export const ThemeSourceProvider = ThemeSourceContext.Provider;

/** The source in force, and its state as it stands. */
export function useThemes(): ThemeState & { source: ModsSource } {
  const source = useContext(ThemeSourceContext);
  const state = useSyncExternalStore(source.subscribe, source.get);
  return { ...state, source };
}
