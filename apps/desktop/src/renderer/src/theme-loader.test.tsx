// The theme loader, against a real document: what it puts on <html>, what it adopts, and that
// nothing it is handed can leave Bridge without a theme.

import { createElement } from "react";
import { flushSync } from "react-dom";
import { createRoot } from "react-dom/client";
import { afterEach, expect, test } from "vitest";
import { createThemeSource } from "@armada/settings";
import type { CatalogueTheme, ThemeMod, ThemeState } from "@armada/settings";

import { createThemeLoader } from "./theme-loader";
import { Themed } from "./theme";

const MOD = (name: string, css: string, enabled = true): ThemeMod => ({ name, title: name, enabled, load: () => Promise.resolve(css) });
const STATE = (active: string, ...mods: ThemeMod[]): ThemeState => ({ active, mods, catalogue: [] });
const BLOCK = (id: string, bg = "tomato") => `[data-theme="${id}"] { --bg-base: ${bg}; }`;
const DUSK_CSS = BLOCK("dusk");
const DUSK = MOD("dusk", DUSK_CSS);

const theme = () => document.documentElement.dataset["theme"];
const bgBase = () => getComputedStyle(document.documentElement).getPropertyValue("--bg-base").trim();

afterEach(() => {
  document.adoptedStyleSheets = [];
  delete document.documentElement.dataset["theme"];
});

test("a built-in theme sets data-theme and adopts nothing", () => {
  const loader = createThemeLoader(document);
  expect(loader.apply("light", null)).toBe("light");
  expect(theme()).toBe("light");
  expect(document.adoptedStyleSheets).toHaveLength(0);
});

test("a theme's CSS is adopted under its own data-theme, and leaves with it", () => {
  const loader = createThemeLoader(document);
  expect(loader.apply("dusk", DUSK_CSS)).toBe("dusk");
  expect(theme()).toBe("dusk");
  expect(document.adoptedStyleSheets).toHaveLength(1);
  expect(bgBase()).toBe("tomato");

  expect(loader.apply("dark", null)).toBe("dark");
  expect(document.adoptedStyleSheets).toHaveLength(0);
  expect(bgBase()).not.toBe("tomato");
});

test("CSS that is empty, unparseable, unscoped or fetches something is Dark", () => {
  const loader = createThemeLoader(document);
  for (const css of ["", "/* nothing */", "not css at all {{{", "body { --bg-base: red; }", '@import url("https://example.com/x.css"); ' + BLOCK("dusk")]) {
    expect(loader.apply("dusk", css)).toBe("dark");
    expect(theme()).toBe("dark");
    expect(document.adoptedStyleSheets).toHaveLength(0);
  }
});

test("a theme with no CSS to apply is Dark", () => {
  expect(createThemeLoader(document).apply("gone", null)).toBe("dark");
});

test("a bad theme does not take down the one before it", () => {
  const loader = createThemeLoader(document);
  loader.apply("dusk", DUSK_CSS);
  loader.apply("dusk", "");
  expect(theme()).toBe("dark");
  expect(document.adoptedStyleSheets).toHaveLength(0);
});

test("clear leaves the document as it was found", () => {
  const loader = createThemeLoader(document);
  loader.apply("dusk", DUSK_CSS);
  loader.clear();
  expect(theme()).toBeUndefined();
  expect(document.adoptedStyleSheets).toHaveLength(0);
});

/** Mounts `Themed` over nothing, returns how to take it down. */
function themed(props: Parameters<typeof Themed>[0]) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  flushSync(() => root.render(createElement(Themed, props)));
  return () => {
    root.unmount();
    host.remove();
  };
}

const settled = () => new Promise((done) => setTimeout(done, 0));
const SHIPPED = (id: string, css: string | (() => Promise<string>)): CatalogueTheme => ({ id, title: id, tone: "dark", load: typeof css === "string" ? () => Promise.resolve(css) : css });

test("a catalogue theme is read when it is chosen, then applied like a mod's", async () => {
  let reads = 0;
  const shipped = SHIPPED("catalogue:nord", () => (reads += 1, Promise.resolve(BLOCK("catalogue:nord", "navy"))));
  const source = createThemeSource(() => ({ mods: [], catalogue: [shipped], active: "dark" }));
  const down = themed({ source });
  expect(reads).toBe(0);
  source.setActive("catalogue:nord");
  await settled();
  expect(reads).toBe(1);
  expect(theme()).toBe("catalogue:nord");
  expect(bgBase()).toBe("navy");
  down();
});

test("a catalogue theme that cannot be read is Dark, in the source as well", async () => {
  const shipped = SHIPPED("catalogue:gone", () => Promise.reject(new Error("no such chunk")));
  const source = createThemeSource(() => ({ mods: [], catalogue: [shipped], active: "catalogue:gone" }));
  const down = themed({ source });
  await settled();
  expect(theme()).toBe("dark");
  expect(source.get().active).toBe("dark");
  down();
});

test("the last choice wins when an earlier one is still being read", async () => {
  let release: (css: string) => void = () => undefined;
  const slow = SHIPPED("catalogue:slow", () => new Promise<string>((done) => (release = done)));
  const source = createThemeSource(() => ({ mods: [], catalogue: [slow], active: "dark" }));
  const down = themed({ source });
  source.setActive("catalogue:slow");
  source.setActive("light");
  release(BLOCK("catalogue:slow"));
  await settled();
  expect(theme()).toBe("light");
  down();
});

test("the source and the picker are told Dark when a mod's CSS is refused", async () => {
  const source = createThemeSource(() => STATE("dusk", MOD("dusk", "")));
  const down = themed({ source });
  await settled();
  expect(theme()).toBe("dark");
  expect(source.get().active).toBe("dark");
  down();
});

test("a mod's CSS is read when its theme is drawn, and a mod that cannot be read is Dark", async () => {
  let reads = 0;
  const read = { ...DUSK, load: () => (reads += 1, Promise.resolve(BLOCK("dusk", "navy"))) };
  const source = createThemeSource(() => STATE("dusk", read));
  const down = themed({ source });
  await settled();
  expect(reads).toBe(1);
  expect(theme()).toBe("dusk");
  expect(bgBase()).toBe("navy");
  down();

  const unreadable = { ...DUSK, load: () => Promise.reject(new Error("did not pass its checks")) };
  const gone = themed({ source: createThemeSource(() => STATE("dusk", unreadable)) });
  await settled();
  expect(theme()).toBe("dark");
  expect(document.adoptedStyleSheets).toHaveLength(0);
  gone();
});

test("a source that keeps the choice is told which theme fell back, and is not made to choose Dark", async () => {
  const told: string[] = [];
  const source = createThemeSource(() => STATE("dusk", MOD("dusk", "")));
  const kept = { ...source, fellBack: (id: string) => void told.push(id), setActive: (id: string) => void told.push(`chose ${id}`) };
  const down = themed({ source: kept });
  await settled();
  expect(theme()).toBe("dark");
  expect(told).toEqual(["dusk"]);
  down();
});

test("the blocks Fleet's checks let a mod write are accepted: bare :root and [data-theme], and its own name", () => {
  const loader = createThemeLoader(document);
  for (const css of [":root { --bg-base: tomato; }", "[data-theme] { --bg-base: tomato; }", `:root, ${BLOCK("dusk").split(" {")[0]} { --bg-base: tomato; }`]) {
    expect(loader.apply("dusk", css)).toBe("dusk");
    expect(bgBase()).toBe("tomato");
  }
  // Anything else is not the theme's: it would restyle Bridge whichever theme is in force.
  for (const css of ["body { --bg-base: red; }", `:root, body { --bg-base: red; }`, BLOCK("other")]) {
    expect(loader.apply("dusk", css)).toBe("dark");
  }
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
