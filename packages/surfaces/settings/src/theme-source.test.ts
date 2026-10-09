import { expect, test } from "vitest";

import { createThemeSource, withoutMods } from "./theme-source";
import type { CatalogueTheme, ThemeMod, ThemeState } from "./theme-source";

const DUSK: ThemeMod = { name: "dusk", title: "Dusk", enabled: true, load: () => Promise.resolve("") };
const SHIPPED: CatalogueTheme = { id: "catalogue:nord", title: "Nord", tone: "dark", load: () => Promise.resolve("") };
const start = (): ThemeState => ({ mods: [DUSK], catalogue: [SHIPPED], active: "dark" });

test("an enabled mod can be chosen; one that is off, or unknown, is Dark", () => {
  const source = createThemeSource(start);
  source.setActive("dusk");
  expect(source.get().active).toBe("dusk");
  source.setActive("gone");
  expect(source.get().active).toBe("dark");
});

test("switching the active mod off falls back to Dark", () => {
  const source = createThemeSource(start);
  source.setActive("dusk");
  source.setEnabled("dusk", false);
  expect(source.get().active).toBe("dark");
});

test("subscribers hear every change, and not after they leave", () => {
  const source = createThemeSource(start);
  let heard = 0;
  const off = source.subscribe(() => (heard += 1));
  source.setActive("light");
  off();
  source.setActive("dark");
  expect(heard).toBe(1);
});

test("safe mode shows no mods and refuses to choose one", () => {
  const source = createThemeSource(start);
  const safe = withoutMods(source);
  expect(safe.get().mods).toEqual([]);
  safe.setActive("dusk");
  expect(source.get().active).toBe("dark");
  safe.setActive("light");
  expect(safe.get().active).toBe("light");
  // The same state until the source changes, so a store reading it does not loop.
  expect(safe.get()).toBe(safe.get());
});

test("safe mode over a mod already chosen reads Dark", () => {
  const source = createThemeSource(() => ({ mods: [DUSK], catalogue: [SHIPPED], active: "dusk" }));
  expect(withoutMods(source).get().active).toBe("dark");
});

test("safe mode keeps the catalogue: it is Bridge's, not a mod's", () => {
  const safe = withoutMods(createThemeSource(start));
  expect(safe.get().catalogue).toEqual([SHIPPED]);
  safe.setActive("catalogue:nord");
  expect(safe.get().active).toBe("catalogue:nord");
});

test("a catalogue theme can be chosen, and an id nothing offers is Dark", () => {
  const source = createThemeSource(start);
  source.setActive("catalogue:nord");
  expect(source.get().active).toBe("catalogue:nord");
  source.setActive("catalogue:nowhere");
  expect(source.get().active).toBe("dark");
});

test("a mod with something wrong with it cannot be chosen, and one already chosen falls to Dark when it breaks", () => {
  const source = createThemeSource(() => ({ mods: [{ ...DUSK, problem: "line 2: not a token" }], catalogue: [], active: "dark" }));
  source.setActive("dusk");
  expect(source.get().active).toBe("dark");
});

test("safe mode passes a fallback on to a source that keeps the choice", () => {
  const told: string[] = [];
  const kept = { ...createThemeSource(start), fellBack: (id: string) => void told.push(id) };
  withoutMods(kept).fellBack?.("catalogue:nord");
  expect(told).toEqual(["catalogue:nord"]);
  expect(withoutMods(createThemeSource(start)).fellBack).toBeUndefined();
});
