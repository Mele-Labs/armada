import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

import { forgetViewed, markViewed, viewedAt } from "./viewed";

/** A window with a storage that remembers. */
function windowWith() {
  const store = new Map<string, string>();
  vi.stubGlobal("window", {
    localStorage: { getItem: (key: string) => store.get(key) ?? null, setItem: (key: string, value: string) => void store.set(key, value), removeItem: (key: string) => void store.delete(key) },
  });
  return store;
}

let store: Map<string, string>;
beforeEach(() => {
  store = windowWith();
  forgetViewed();
});
afterEach(() => vi.unstubAllGlobals());

describe("when he was last in a Session", () => {
  test("a Session he never was in reads 0", () => {
    expect(viewedAt("s1")).toBe(0);
  });

  test("is kept per Session and in storage, so a restart still has it", () => {
    markViewed("s1", 1000);
    markViewed("s2", 2000);
    expect(viewedAt("s1")).toBe(1000);
    expect(viewedAt("s2")).toBe(2000);
    expect(JSON.parse(store.get("armada.bridge.session-viewed") ?? "{}")).toEqual({ s1: 1000, s2: 2000 });
  });

  test("never goes back: an earlier mark does not undo a later one", () => {
    markViewed("s1", 2000);
    markViewed("s1", 1000);
    expect(viewedAt("s1")).toBe(2000);
  });
});
