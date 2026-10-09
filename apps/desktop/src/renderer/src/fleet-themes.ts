// The themes Bridge offers, over Fleet: its mod list, the mods' stylesheets as Fleet checks them,
// and the theme preference it keeps. A `ModsSource` like the mock's, so `Themed` and every surface
// read it unchanged. `docs/concepts/mods.md`.
//
// **What is chosen and what is shown are two things.** The preference is the owner's word and
// outlives a mod that is off, broken or briefly unreadable; what the window shows is Dark whenever
// that word names a theme Bridge may not draw. The word is forgotten (saved as Dark) only when
// the owner can no longer have meant it: the mod is gone or switched off. A mod that merely fails a
// check returns by itself once it passes, and a mod that appears later under a forgotten name is not
// chosen by having appeared.

import type { ModChecked, Outcome, SavePreference } from "@armada/protocol";
import { BUILT_IN_THEMES, offered } from "@armada/settings";
import type { CatalogueTheme, ModsSource, ThemeMod, ThemeState } from "@armada/settings";
import { refusalWords } from "@armada/screens/src/refusal-words";

import type { BridgeState } from "../../shared/bridge";
import { askToTell } from "./tell";

const DARK = "dark";
const CATALOGUE = "catalogue:";

type Facts = Pick<BridgeState, "mods" | "preferences">;

/** What Bridge reaches Fleet through: `window.armada` in the app, a fake in a test. */
export type FleetThemes = {
  state: () => Promise<Facts>;
  subscribe: (on: (facts: Facts) => void) => () => void;
  validateMod: (name: string) => Promise<ModChecked | null>;
  setModEnabled: (name: string, enabled: boolean) => Promise<Outcome>;
  promoteMod: (name: string) => Promise<Outcome>;
  savePreference: (save: SavePreference) => Promise<Outcome>;
};

const isBuiltIn = (id: string) => (BUILT_IN_THEMES as readonly string[]).includes(id);

/** How a theme is previewed: Dark and Light by id, and a mod from the stylesheet Fleet checked. */
export type Colours = {
  builtIn: Readonly<Record<string, readonly string[]>>;
  /** Throws where the sheet gives no colour for a token. */
  ofMod: (css: string) => readonly string[];
};

export function createFleetThemes(fleet: FleetThemes, catalogue: readonly CatalogueTheme[], colours: Colours, tell: (sentence: string) => void = askToTell): ModsSource {
  let facts: Facts["mods"] = null;
  let chosen = DARK;
  // The id the loader could not draw: shown as Dark until the list changes, and never forgotten.
  let declined: string | null = null;
  let saving = 0;
  let seen = "";
  const branches = new Map<string, string>();
  // A mod's strip, kept with the `changed_at` it was read at, so an edited mod is read again.
  const strips = new Map<string, { at: string; swatch: readonly string[] }>();
  const reading = new Set<string>();
  const stamp = (name: string) => facts?.mods.find((row) => row.name === name)?.changed_at ?? "";

  /** Keep the strip a checked stylesheet gives, if it gives one. Returns whether anything moved. */
  const keep = (name: string, css: string): boolean => {
    try {
      const swatch = colours.ofMod(css);
      const before = strips.get(name);
      strips.set(name, { at: stamp(name), swatch });
      return before === undefined || before.at !== stamp(name) || before.swatch.join() !== swatch.join();
    } catch {
      return false;
    }
  };
  const listeners = new Set<() => void>();

  /** The stylesheet Fleet checked, or a rejection saying why there is none. */
  const loadOf = (name: string) => async (): Promise<string> => {
    const checked = await fleet.validateMod(name);
    if (checked === null) throw new Error(`${name} could not be checked`);
    if (!checked.valid || checked.css === undefined) throw new Error(checked.problems[0] ?? `${name} did not pass its checks`);
    if (keep(name, checked.css)) refresh();
    return checked.css;
  };

  const modsNow = (): ThemeMod[] =>
    (facts?.mods ?? []).map((row) => ({
      name: row.name,
      title: row.name,
      enabled: row.enabled,
      ...(row.valid ? {} : { problem: row.reason ?? "Did not pass its checks" }),
      ...(branches.has(row.name) ? { branch: branches.get(row.name)! } : {}),
      ...(strips.get(row.name)?.at === (row.changed_at ?? "") ? { swatch: strips.get(row.name)!.swatch } : {}),
      load: loadOf(row.name),
    }));

  const stateNow = (): ThemeState => {
    const mods = modsNow();
    const drawable = isBuiltIn(chosen) || catalogue.some((one) => one.id === chosen) || mods.some((mod) => mod.name === chosen && offered(mod));
    return { mods, catalogue, active: drawable && declined !== chosen ? chosen : DARK, swatches: colours.builtIn };
  };

  let held = stateNow();
  function refresh() {
    held = stateNow();
    listeners.forEach((on) => on());
  }

  /** The strip of each mod the picker will offer, read ahead of its being chosen: a check, and nothing is applied. */
  const preview = () => {
    for (const row of facts?.mods ?? []) {
      const key = `${row.name}@${row.changed_at ?? ""}`;
      if (!row.valid || !row.enabled || strips.get(row.name)?.at === (row.changed_at ?? "") || reading.has(key)) continue;
      reading.add(key);
      fleet.validateMod(row.name).then(
        (checked) => {
          reading.delete(key);
          if (checked?.valid === true && checked.css !== undefined && keep(row.name, checked.css)) refresh();
        },
        () => void reading.delete(key),
      );
    }
  };

  /** Save the theme word. A refusal puts the old word back and, for the owner's own press, says why. */
  const save = async (id: string, quiet: boolean): Promise<void> => {
    const before = chosen;
    saving += 1;
    chosen = id;
    declined = null;
    refresh();
    const outcome = await fleet.savePreference({ name: "theme", value: false, text: id });
    saving -= 1;
    if (outcome.ok) return;
    if (chosen === id) {
      chosen = before;
      refresh();
    }
    if (!quiet) tell(refusalWords(outcome));
  };

  const ingest = (next: Facts) => {
    const theme = next.preferences.theme ?? DARK;
    const key = JSON.stringify([next.mods, theme]);
    if (key === seen) return;
    seen = key;
    // A layout mod is not a theme, and the Mods surface lists it from the layout source.
    facts = next.mods === null ? null : { mods: next.mods.mods.filter((row) => row.kind !== "layout") };
    declined = null;
    if (saving === 0) chosen = theme;
    refresh();
    preview();
    // A mod the owner chose that is no longer there, or no longer on, is not chosen any more.
    const gone = !isBuiltIn(chosen) && !chosen.startsWith(CATALOGUE) && facts !== null && !facts.mods.some((row) => row.name === chosen && row.enabled);
    if (gone && saving === 0) void save(DARK, true);
  };

  fleet.state().then(ingest, () => undefined);
  fleet.subscribe(ingest);

  return {
    get: () => held,
    subscribe: (on) => {
      listeners.add(on);
      return () => void listeners.delete(on);
    },
    setActive: (id) => {
      const offeredNow = isBuiltIn(id) || catalogue.some((one) => one.id === id) || held.mods.some((mod) => mod.name === id && offered(mod));
      void save(offeredNow ? id : DARK, false);
    },
    fellBack: (id) => {
      if (id !== chosen) return;
      declined = id;
      refresh();
    },
    setEnabled: (name, enabled) => {
      void fleet.setModEnabled(name, enabled).then((outcome) => {
        if (!outcome.ok) tell(refusalWords(outcome));
      });
    },
    promote: (name) => {
      void fleet.promoteMod(name).then((outcome) => {
        if (!outcome.ok) return tell(refusalWords(outcome));
        if (outcome.modPromoted === undefined) return;
        branches.set(name, outcome.modPromoted.branch);
        refresh();
      });
    },
  };
}
