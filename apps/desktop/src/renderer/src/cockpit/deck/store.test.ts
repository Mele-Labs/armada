// The deck's store: the ways of reading that outlive a restart are remembered, a window with nothing stored opens plain,
// and the calls still to be opened come newest first by the Item's own arrival.

import { afterEach, beforeEach, expect, test, vi } from "vitest";

import type { Item } from "../../Dashboard";
import type { Ship, Subject } from "./ship";
import { arrivedOf, deckNow, forgetDeck, incomingOf, keepOf, listOf, setJobView, setView, toggleChecks, toggleNeeds, toggleOnly } from "./store";

const kept = new Map<string, string>();

beforeEach(() => {
  kept.clear();
  vi.stubGlobal("window", { localStorage: { getItem: (key: string) => kept.get(key) ?? null, setItem: (key: string, value: string) => void kept.set(key, value) } });
  forgetDeck();
});
afterEach(() => {
  vi.unstubAllGlobals();
  forgetDeck();
});

test("a window with nothing stored opens on the map, with the mission and no filter", () => {
  const now = deckNow();
  expect([now.view, now.jobView, now.only, now.needs, now.checks]).toEqual(["map", "mission", undefined, false, false]);
});

test("the view, how a Job draws its run and the filters are remembered across a new window", () => {
  setView("timeline");
  setJobView("canvas");
  toggleOnly("jobs");
  toggleNeeds();
  toggleChecks();
  forgetDeck();

  const now = deckNow();
  expect([now.view, now.jobView, now.only, now.needs, now.checks]).toEqual(["timeline", "canvas", "jobs", true, true]);

  toggleOnly("jobs");
  toggleChecks();
  forgetDeck();
  expect([deckNow().only, deckNow().needs, deckNow().checks]).toEqual([undefined, true, false]);
});

test("what is open, the cursor and the visited calls are not remembered", () => {
  setView("timeline");
  forgetDeck();
  expect(deckNow().sel).toBeUndefined();
  expect(deckNow().visited.size).toBe(0);
});

test("storage that cannot be read or written is the plain default and does not stop the deck", () => {
  vi.stubGlobal("window", {
    get localStorage(): never {
      throw new Error("denied");
    },
  });
  forgetDeck();
  expect(deckNow().view).toBe("map");
  expect(() => setView("timeline")).not.toThrow();
  expect(deckNow().view).toBe("timeline");
});

const call = (key: string, at: string | undefined): Item => ({ key, at }) as Item;

test("the calls to be opened come newest first by the Item's own arrival, the order Fleet carries them breaking a tie", () => {
  const calls = [call("old", "2026-10-14T09:00:00Z"), call("new", "2026-10-14T11:00:00Z"), call("undated-a", undefined), call("tie-a", "2026-10-14T10:00:00Z"), call("tie-b", "2026-10-14T10:00:00Z")];

  expect(incomingOf(calls, new Map(), new Set()).map((one) => one.key)).toEqual(["new", "tie-a", "tie-b", "old", "undated-a"]);
  expect(arrivedOf(call("x", undefined))).toBe(0);
});

test("a call that is answered or visited leaves the tray", () => {
  const calls = [call("a", "2026-10-14T09:00:00Z"), call("b", "2026-10-14T10:00:00Z"), call("c", "2026-10-14T11:00:00Z")];
  expect(incomingOf(calls, new Map([["c", 1]]), new Set(["b"])).map((one) => one.key)).toEqual(["a"]);
});

// ---- Checks, a plain filter ----------------------------------------------------------------------------

const subject = (kind: "job" | "session", id: string): Subject =>
  ({ key: `${kind}:${id}`, kind, id, title: id, root: "/r", repo: "r", mark: { Glyph: () => null, said: "", tone: "calm" }, phase: "working", needs: false, alarm: false, peek: [], stoppable: false, repliable: false, mergeable: false }) as unknown as Subject;
const ship = (running: string[]): Ship => ({ subjects: [subject("job", "a"), subject("job", "b"), subject("session", "c")], running: new Set(running) }) as unknown as Ship;

test("Checks narrows the list to the subjects with a Check running now, and the feed and the stage keep the same ones", () => {
  toggleChecks();
  const listed = listOf(ship(["job:b", "session:c"]), deckNow());
  expect(listed.map((one) => one.key).sort()).toEqual(["job:b", "session:c"]);
  const keep = keepOf(listed, deckNow());
  expect(keep({ kind: "job", id: "b" })).toBe(true);
  expect(keep({ kind: "job", id: "a" })).toBe(false);
  expect(keep({ kind: "session", id: "c" })).toBe(true);
  expect(keep({ kind: "main" })).toBe(false);
});

test("with no filter on, the feed and the stage keep every subject the list draws and main's events too", () => {
  const listed = listOf(ship([]), deckNow());
  expect(listed).toHaveLength(3);
  const keep = keepOf(listed, deckNow());
  expect(keep({ kind: "job", id: "a" })).toBe(true);
  expect(keep({ kind: "main" })).toBe(true);
});
