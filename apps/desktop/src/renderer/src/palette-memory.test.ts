import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

import { forgetPaletteMemory, nextOpens, paletteMemory, PALETTE_KEY, remember, rowOf, setAlias, setHotkey, togglePin } from "./palette-memory";

/** A window with a storage that remembers, and one that has been closed and opened again. */
function windowWith(store = new Map<string, string>()) {
  vi.stubGlobal("window", {
    localStorage: { getItem: (key: string) => store.get(key) ?? null, setItem: (key: string, value: string) => void store.set(key, value) },
    addEventListener: () => undefined,
    removeEventListener: () => undefined,
  });
  forgetPaletteMemory();
  return store;
}

let store: Map<string, string>;
beforeEach(() => {
  store = windowWith();
});
afterEach(() => vi.unstubAllGlobals());

/** Closing the window and opening it again: nothing held in memory, only the storage. */
const relaunch = () => windowWith(store);

describe("pins", () => {
  test("a pin is kept, and pressing it again lets it go", () => {
    togglePin("session:q1");
    expect(paletteMemory().pinned).toEqual(["session:q1"]);
    togglePin("session:q1");
    expect(paletteMemory().pinned).toEqual([]);
  });

  test("pins come back after the app is closed and opened again, in the order they were pinned", () => {
    togglePin("session:q1");
    togglePin("cmd:grafana");
    relaunch();
    expect(paletteMemory().pinned).toEqual(["session:q1", "cmd:grafana"]);
  });
});

describe("recent", () => {
  test("the row opened last goes to the head, and a row opened again moves up rather than repeating", () => {
    remember("pull:armada:1");
    remember("nav:reviews");
    remember("pull:armada:1");
    expect(paletteMemory().recent).toEqual(["pull:armada:1", "nav:reviews"]);
  });

  test("six are kept, the oldest first to go", () => {
    for (const n of [1, 2, 3, 4, 5, 6, 7]) remember(`task:${n}`);
    expect(paletteMemory().recent).toEqual(["task:7", "task:6", "task:5", "task:4", "task:3", "task:2"]);
  });

  test("recent comes back after the app is closed and opened again", () => {
    remember("cmd:audit");
    relaunch();
    expect(paletteMemory().recent).toEqual(["cmd:audit"]);
  });

  test("a pin and a press on the row that stays are not rows opened", () => {
    expect(rowOf({ of: "on", target: "session:q1", action: "pin" })).toBeUndefined();
    expect(rowOf({ of: "on", target: "session:q1", action: "unpin" })).toBeUndefined();
    expect(rowOf({ of: "on", target: "session:q1", action: "pause" })).toBe("session:q1");
    expect(rowOf({ of: "pull", id: "armada:1899" })).toBe("pull:armada:1899");
  });
});

describe("what the empty query shows", () => {
  test("pinned and recent until it is changed, then each of the others in turn, and kept", () => {
    expect(paletteMemory().opens).toBe("pinned");
    nextOpens();
    expect(paletteMemory().opens).toBe("page");
    nextOpens();
    expect(paletteMemory().opens).toBe("everything");
    relaunch();
    expect(paletteMemory().opens).toBe("everything");
    nextOpens();
    expect(paletteMemory().opens).toBe("pinned");
  });
});

describe("aliases and keys per row", () => {
  test("an alias and a key are kept for a row, and an empty one lets go", () => {
    setAlias("cmd:audit", "aud");
    setHotkey("cmd:audit", "⇧⌘A");
    relaunch();
    expect(paletteMemory().aliases).toEqual({ "cmd:audit": "aud" });
    expect(paletteMemory().hotkeys).toEqual({ "cmd:audit": "⇧⌘A" });
    setAlias("cmd:audit", "  ");
    setHotkey("cmd:audit", "");
    expect(paletteMemory().aliases).toEqual({});
    expect(paletteMemory().hotkeys).toEqual({});
  });
});

describe("a storage that holds something else", () => {
  test("is read as nothing kept, and what is whole is kept", () => {
    store.set(PALETTE_KEY, JSON.stringify({ pinned: ["session:q1", 4], recent: "nope", opens: "sideways", aliases: { a: "x", b: 2 } }));
    relaunch();
    expect(paletteMemory()).toEqual({ pinned: ["session:q1"], recent: [], opens: "pinned", aliases: { a: "x" }, hotkeys: {} });
    store.set(PALETTE_KEY, "{not json");
    relaunch();
    expect(paletteMemory().pinned).toEqual([]);
  });
});
