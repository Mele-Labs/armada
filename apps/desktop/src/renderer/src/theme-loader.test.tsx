// The theme loader, against a real document: what it puts on <html>, what it adopts, and that
// nothing it is handed can leave Bridge without a theme.

import { createElement } from "react";
import { flushSync } from "react-dom";
import { createRoot } from "react-dom/client";
import { afterEach, expect, test } from "vitest";
import { createThemeSource } from "@armada/settings";
import type { ThemeMod, ThemeState } from "@armada/settings";

import { createThemeLoader } from "./theme-loader";
import { Themed } from "./theme";

const MOD = (name: string, css: string, enabled = true): ThemeMod => ({ name, title: name, css, enabled, promoted: false });
const STATE = (active: string, ...mods: ThemeMod[]): ThemeState => ({ active, mods });
const DUSK = MOD("dusk", '[data-theme="dusk"] { --bg-base: #1A1822; }');

const theme = () => document.documentElement.dataset["theme"];
const bgBase = () => getComputedStyle(document.documentElement).getPropertyValue("--bg-base").trim();

afterEach(() => {
  document.adoptedStyleSheets = [];
  delete document.documentElement.dataset["theme"];
});

test("a built-in theme sets data-theme and adopts nothing", () => {
  const loader = createThemeLoader(document);
  expect(loader.apply(STATE("light"))).toBe("light");
  expect(theme()).toBe("light");
  expect(document.adoptedStyleSheets).toHaveLength(0);
});

test("an enabled mod's CSS is adopted under its own data-theme, and leaves with it", () => {
  const loader = createThemeLoader(document);
  expect(loader.apply(STATE("dusk", DUSK))).toBe("dusk");
  expect(theme()).toBe("dusk");
  expect(document.adoptedStyleSheets).toHaveLength(1);
  expect(bgBase()).toBe("#1A1822");

  expect(loader.apply(STATE("dark", DUSK))).toBe("dark");
  expect(document.adoptedStyleSheets).toHaveLength(0);
  expect(bgBase()).not.toBe("#1A1822");
});

test("CSS that is empty, unparseable, unscoped or fetches something is Dark", () => {
  const loader = createThemeLoader(document);
  for (const css of ["", "/* nothing */", "not css at all {{{", "body { --bg-base: red; }", '@import url("https://example.com/x.css"); [data-theme="dusk"] { --bg-base: red; }']) {
    expect(loader.apply(STATE("dusk", MOD("dusk", css)))).toBe("dark");
    expect(theme()).toBe("dark");
    expect(document.adoptedStyleSheets).toHaveLength(0);
  }
});

test("a mod that is off, or not there, is Dark", () => {
  const loader = createThemeLoader(document);
  expect(loader.apply(STATE("dusk", MOD("dusk", DUSK.css, false)))).toBe("dark");
  expect(loader.apply(STATE("gone"))).toBe("dark");
});

test("a bad mod does not take down the theme before it", () => {
  const loader = createThemeLoader(document);
  loader.apply(STATE("dusk", DUSK));
  loader.apply(STATE("dusk", MOD("dusk", "")));
  expect(theme()).toBe("dark");
  expect(document.adoptedStyleSheets).toHaveLength(0);
});

test("clear leaves the document as it was found", () => {
  const loader = createThemeLoader(document);
  loader.apply(STATE("dusk", DUSK));
  loader.clear();
  expect(theme()).toBeUndefined();
  expect(document.adoptedStyleSheets).toHaveLength(0);
});

/** Mounts `Themed` over nothing, returns how to take it down. */
function themed(props: Omit<Parameters<typeof Themed>[0], "children">) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  flushSync(() => root.render(createElement(Themed, props, null)));
  return () => {
    root.unmount();
    host.remove();
  };
}

test("the source and the picker are told Dark when a mod's CSS is refused", () => {
  const source = createThemeSource(() => STATE("dusk", MOD("dusk", "")));
  // `createThemeSource` keeps an enabled mod's id; the loader is what refuses its CSS.
  const down = themed({ source });
  expect(theme()).toBe("dark");
  expect(source.get().active).toBe("dark");
  down();
});

test("safe mode loads without mods", () => {
  const source = createThemeSource(() => STATE("dusk", DUSK));
  const down = themed({ source, noMods: true });
  expect(theme()).toBe("dark");
  expect(document.adoptedStyleSheets).toHaveLength(0);
  // The mod is still on the machine; it is only not loaded.
  expect(source.get().active).toBe("dusk");
  down();
});

test("a source that throws leaves Dark, and Bridge mounted", () => {
  const broken = { ...createThemeSource(), get: (): ThemeState => { throw new Error("the mod folder is unreadable"); } };
  const down = themed({ source: broken });
  expect(theme()).toBe("dark");
  down();
});
