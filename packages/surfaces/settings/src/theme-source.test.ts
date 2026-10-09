import { expect, test } from "vitest";

import { createThemeSource, withoutMods } from "./theme-source";
import type { ThemeMod, ThemeState } from "./theme-source";

const NORD: ThemeMod = { name: "nord", title: "Nord", css: "", enabled: true, promoted: false };
const start = (): ThemeState => ({ mods: [NORD], active: "dark" });

test("an enabled mod can be chosen; one that is off, or unknown, is Dark", () => {
  const source = createThemeSource(start);
  source.setActive("nord");
  expect(source.get().active).toBe("nord");
  source.setActive("gone");
  expect(source.get().active).toBe("dark");
});

test("switching the active mod off falls back to Dark", () => {
  const source = createThemeSource(start);
  source.setActive("nord");
  source.setEnabled("nord", false);
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
  safe.setActive("nord");
  expect(source.get().active).toBe("dark");
  safe.setActive("light");
  expect(safe.get().active).toBe("light");
  // The same state until the source changes, so a store reading it does not loop.
  expect(safe.get()).toBe(safe.get());
});

test("safe mode over a mod already chosen reads Dark", () => {
  const source = createThemeSource(() => ({ mods: [NORD], active: "nord" }));
  expect(withoutMods(source).get().active).toBe("dark");
});
