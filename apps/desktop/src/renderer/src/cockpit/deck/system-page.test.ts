// The System tab opens the last page used there, and the first where none is remembered or the one remembered is gone.

import { afterEach, beforeEach, expect, test, vi } from "vitest";

const kept = new Map<string, string>();

beforeEach(() => {
  kept.clear();
  vi.resetModules();
  vi.stubGlobal("window", { localStorage: { getItem: (key: string) => kept.get(key) ?? null, setItem: (key: string, value: string) => void kept.set(key, value) } });
});
afterEach(() => vi.unstubAllGlobals());

test("the System tab opens the last page used there, and the first where none is remembered or the one remembered is gone", async () => {
  const { rememberSystemPage, systemPageOf } = await import("./system-page");
  const pages = ["kit", "mods", "settings"];
  expect(systemPageOf(pages, "kit")).toBe("kit");
  rememberSystemPage("settings");
  expect(systemPageOf(pages, "kit")).toBe("settings");
  rememberSystemPage("guides");
  expect(systemPageOf(pages, "kit")).toBe("kit");
});
