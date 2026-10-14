// The preview switch is a remembered view setting: off where nothing is stored, remembered across a window, and named by the
// palette's one row for what pressing it does.

import { afterEach, beforeEach, expect, test, vi } from "vitest";

const kept = new Map<string, string>();

beforeEach(() => {
  kept.clear();
  vi.resetModules();
  vi.stubGlobal("window", { localStorage: { getItem: (key: string) => kept.get(key) ?? null, setItem: (key: string, value: string) => void kept.set(key, value) } });
});
afterEach(() => vi.unstubAllGlobals());

test("a window with nothing stored opens on the classic Cockpit", async () => {
  const { deckPreview } = await import("./preview");
  expect(deckPreview()).toBe(false);
});

test("the switch is remembered by the next window", async () => {
  const first = await import("./preview");
  first.setDeckPreview(true);
  vi.resetModules();
  const next = await import("./preview");
  expect(next.deckPreview()).toBe(true);
  next.setDeckPreview(false);
  vi.resetModules();
  expect((await import("./preview")).deckPreview()).toBe(false);
});

test("the palette row names what it switches to", async () => {
  const { previewLabel } = await import("./preview");
  expect([previewLabel(false), previewLabel(true)]).toEqual(["Flight deck", "Classic Cockpit"]);
});

test("the System tab opens the last page used there, and the first where none is remembered or the one remembered is gone", async () => {
  const { rememberSystemPage, systemPageOf } = await import("./preview");
  const pages = ["kit", "mods", "settings"];
  expect(systemPageOf(pages, "kit")).toBe("kit");
  rememberSystemPage("settings");
  expect(systemPageOf(pages, "kit")).toBe("settings");
  rememberSystemPage("guides");
  expect(systemPageOf(pages, "kit")).toBe("kit");
});
