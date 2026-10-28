// The deck's store: the ways of reading that outlive a restart are remembered, a window with nothing stored opens plain,
// and the calls still to be opened come newest first by the Item's own arrival.

import { afterEach, beforeEach, expect, test, vi } from "vitest";

import type { Item } from "../../Dashboard";
import { DECK_ORDER_KEY } from "../../remembered-views";
import type { Ship, Subject } from "./ship";
import { arrivedOf, camera, deckNow, forgetDeck, incomingOf, keepOf, listOf, moveCursor, moveRow, openCall, placeNew, select, setScope, setView, toggleChecks, toggleNeeds, wide, withoutOpenSession } from "./store";

const panelFor = vi.hoisted(() => vi.fn(() => true));
vi.mock("../../shared-panel/store", () => ({ openPullRequestInPanel: panelFor }));

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

test("a window with nothing stored opens on the map, with no filter", () => {
  const now = deckNow();
  expect([now.view, now.needs, now.checks]).toEqual(["map", false, false]);
});

test("the view and the filters are remembered across a new window", () => {
  setView("timeline");
  toggleNeeds();
  toggleChecks();
  forgetDeck();

  const now = deckNow();
  expect([now.view, now.needs, now.checks]).toEqual(["timeline", true, true]);

  toggleChecks();
  forgetDeck();
  expect([deckNow().needs, deckNow().checks]).toEqual([true, false]);
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

test("a Session raises one call however much it asks, the newest standing for the rest", () => {
  const calls = [
    call("session:s1:q1", "2026-10-14T09:00:00Z"),
    call("session:s1:q2", "2026-10-14T11:00:00Z"),
    call("session:s1:w1", "2026-10-14T10:00:00Z"),
    call("session:s2:q1", "2026-10-14T08:00:00Z"),
    call("main:/r", "2026-10-14T06:00:00Z"),
  ];

  expect(incomingOf(calls, new Map(), new Set()).map((one) => one.key)).toEqual(["session:s1:q2", "session:s2:q1", "main:/r"]);
  // An answered newest gives the Session's next call its place.
  expect(incomingOf(calls, new Map([["session:s1:q2", 1]]), new Set()).map((one) => one.key)[0]).toBe("session:s1:w1");
});

test("a seen call stays while its Session still needs the person, and a seen one from elsewhere leaves", () => {
  const calls = [call("session:s1:q1", "2026-10-14T09:00:00Z"), call("main:/r", "2026-10-14T10:00:00Z")];
  const seen = new Set(["session:s1:q1", "main:/r"]);
  expect(incomingOf(calls, new Map(), seen, new Set(["session:s1"])).map((one) => one.key)).toEqual(["session:s1:q1"]);
  expect(incomingOf(calls, new Map(), seen, new Set())).toEqual([]);
});

test("a call that is answered or visited leaves the tray", () => {
  const calls = [call("a", "2026-10-14T09:00:00Z"), call("b", "2026-10-14T10:00:00Z"), call("c", "2026-10-14T11:00:00Z")];
  expect(incomingOf(calls, new Map([["c", 1]]), new Set(["b"])).map((one) => one.key)).toEqual(["a"]);
});

// ---- Checks, a plain filter ----------------------------------------------------------------------------

const subject = (kind: "session", id: string): Subject =>
  ({ key: `${kind}:${id}`, kind, id, title: id, root: "/r", repo: "r", mark: { Glyph: () => null, said: "", tone: "calm" }, phase: "working", needs: false, alarm: false, peek: [], stoppable: false, repliable: false, mergeable: false }) as unknown as Subject;
const ship = (running: string[]): Ship => ({ subjects: [subject("session", "a"), subject("session", "b"), subject("session", "c")], running: new Set(running) }) as unknown as Ship;

test("Checks narrows the list to the subjects with a Check running now, and the feed and the stage keep the same ones", () => {
  toggleChecks();
  const listed = listOf(ship(["session:b", "session:c"]), deckNow());
  expect(listed.map((one) => one.key).sort()).toEqual(["session:b", "session:c"]);
  const keep = keepOf(listed, deckNow());
  expect(keep({ kind: "session", id: "b" })).toBe(true);
  expect(keep({ kind: "session", id: "a" })).toBe(false);
  expect(keep({ kind: "session", id: "c" })).toBe(true);
  expect(keep({ kind: "main" })).toBe(false);
});

test("with no filter on, the feed and the stage keep every subject the list draws and main's events too", () => {
  const listed = listOf(ship([]), deckNow());
  expect(listed).toHaveLength(3);
  const keep = keepOf(listed, deckNow());
  expect(keep({ kind: "session", id: "a" })).toBe(true);
  expect(keep({ kind: "main" })).toBe(true);
});

test("moving the cursor to the row it is on tells nobody", () => {
  moveCursor("session:a");
  const before = deckNow();
  moveCursor("session:a");
  expect(deckNow()).toBe(before);
  moveCursor("session:b");
  expect(deckNow().cursor).toBe("session:b");
});

test("closing what is open forgets the map's camera and has the map drawn whole", () => {
  select({ kind: "session", key: "session:a" });
  camera.current = { x: 1, y: 2, eye: 3, tilt: true };
  select(undefined);
  expect(camera.current).toBeUndefined();
  expect(wide.now).toBe(true);
});

test("selecting with nothing open to close leaves the camera and the map as they were", () => {
  camera.current = { x: 1, y: 2, eye: 3, tilt: true };
  select(undefined);
  expect(camera.current).toEqual({ x: 1, y: 2, eye: 3, tilt: true });
  expect(wide.now).toBe(false);
});

test("the open Session's own calls are cards in its thread and do not call from its row, the others' stay", () => {
  const calls = [call("session:s1:q1", "2026-10-14T09:00:00Z"), call("session:s2:q1", "2026-10-14T10:00:00Z"), call("main:/r", "2026-10-14T11:00:00Z")];
  const keys = (sel: Parameters<typeof withoutOpenSession>[1]) => withoutOpenSession(calls, sel).map((one) => one.key);
  expect(keys({ kind: "session", key: "session:s1" })).toEqual(["session:s2:q1", "main:/r"]);
  expect(keys(undefined)).toEqual(["session:s1:q1", "session:s2:q1", "main:/r"]);
  // Only an open Session takes its calls: a call open on the stage leaves every call where it was.
  expect(keys({ kind: "call", key: "session:s1" })).toHaveLength(3);
});

test("opening a pull request's call draws the side panel, and nothing on the deck's stage", () => {
  const ship = { calls: [], subjects: [], prs: [{ number: 7, url: "https://example.test/7" }] } as unknown as Ship;
  const asked = openCall(ship, call("pull:7", undefined));
  expect(asked).toBe(false);
  expect(panelFor.mock.calls).toEqual([[{ number: 7, url: "https://example.test/7" }]]);
  expect(deckNow().sel).toBeUndefined();
});

// ---- Scope and search ----------------------------------------------------------------------------------

const past = (id: string, how: "quiet" | "ended", at: number, older = false): Subject => ({ ...subject("session", id), past: how, at, ...(older ? { older: true } : {}) }) as Subject;
const scoped = (): Ship =>
  ({
    subjects: [subject("session", "live-two"), subject("session", "live"), past("quiet-new", "quiet", 300), past("ended-old", "ended", 100), past("ended-new", "ended", 200), past("ended-ancient", "ended", 1, true)],
    running: new Set(),
    mains: [],
    prs: [],
    calls: [],
  }) as unknown as Ship;
const scope = (which: "active" | "quiet" | "ended" | "all", rest: { found?: ReadonlyMap<string, unknown>; open?: string } = {}) => listOf(scoped(), { ...deckNow(), scope: which, ...rest }).map((one) => one.id);

test("Active lists the live Sessions and no past one", () => {
  expect(scope("active")).not.toContain("quiet-new");
  expect(scope("active").sort()).toEqual(["live", "live-two"]);
});

test("Quiet and Ended list their own Sessions, newest first, and an ended one past the week is left off", () => {
  expect(scope("quiet")).toEqual(["quiet-new"]);
  expect(scope("ended")).toEqual(["ended-new", "ended-old"]);
});

test("All lists the live ones and then every past one, newest first", () => {
  const all = scope("all");
  expect(all.slice(0, 2).sort()).toEqual(["live", "live-two"]);
  expect(all.slice(2)).toEqual(["quiet-new", "ended-new", "ended-old"]);
});

test("a search narrows the list to what it found, and it is the only way to a Session past the week", () => {
  const found = new Map([["session:ended-ancient", {}], ["session:live", {}]]);
  expect(scope("active", { found })).toEqual(["live"]);
  expect(scope("ended", { found })).toEqual(["ended-ancient"]);
  expect(scope("all", { found }).sort()).toEqual(["ended-ancient", "live"]);
});

test("the Job open on the stage stays in the list, whatever the scope", () => {
  expect(scope("ended", { open: "session:quiet-new" })).toEqual(["ended-new", "ended-old", "quiet-new"]);
  expect(scope("active", { open: "session:ended-old" })).toContain("ended-old");
});

test("the scope is remembered across a new window, and choosing one starts the list from its first page", () => {
  setScope("ended");
  forgetDeck();
  expect(deckNow().scope).toBe("ended");
  expect(deckNow().limit).toBe(50);
});

test("a narrowed scope or a search keeps main's events off the feed and the stage", () => {
  const listed = listOf(scoped(), { ...deckNow(), scope: "ended" });
  expect(keepOf(listed, { ...deckNow(), scope: "ended" })({ kind: "main" })).toBe(false);
  expect(keepOf(listed, { ...deckNow(), query: "x" })({ kind: "main" })).toBe(false);
  expect(keepOf(listed, { ...deckNow(), scope: "active", query: " " })({ kind: "main" })).toBe(true);
});

// ---- Linked Sessions, in Work --------------------------------------------------------------------------

test("a Job started from another stands right under its parent, whether or not it was handed a plan group", () => {
  const child = (id: string, linked: Subject["linked"]): Subject => ({ ...subject("session", id), linked });
  const all = [subject("session", "p"), subject("session", "j"), child("grouped", { parent: "session:p", group: "Names" }), child("plain", { parent: "session:p" })];
  const listed = listOf({ subjects: all, running: new Set() } as unknown as Ship, deckNow());
  expect(listed.map((one) => one.key)).toEqual(["session:p", "session:grouped", "session:plain", "session:j"]);
});

test("a Job started from a Job that was itself started from another stands under its own parent", () => {
  const all = [subject("session", "p"), { ...subject("session", "k"), linked: { parent: "session:p" } }, { ...subject("session", "g"), linked: { parent: "session:k" } }, subject("session", "j")];
  const listed = listOf({ subjects: all, running: new Set() } as unknown as Ship, deckNow());
  expect(listed.map((one) => one.key)).toEqual(["session:p", "session:k", "session:g", "session:j"]);
  expect(listed.map((one) => one.linked?.depth)).toEqual([undefined, 1, 2, undefined]);
});

test("a parent's line runs on past each of its Jobs but the last, and past their own Jobs to reach it", () => {
  const all = [subject("session", "p"), { ...subject("session", "k"), linked: { parent: "session:p" } }, { ...subject("session", "g"), linked: { parent: "session:k" } }, { ...subject("session", "m"), linked: { parent: "session:p" } }];
  const listed = listOf({ subjects: all, running: new Set() } as unknown as Ship, deckNow());
  expect(listed.map((one) => [one.key, one.linked?.rails])).toEqual([["session:p", undefined], ["session:k", [1]], ["session:g", [1]], ["session:m", undefined]]);
});

test("a loop of started Jobs is listed once each and never hangs the list", () => {
  const all = [{ ...subject("session", "a"), linked: { parent: "session:b" } }, { ...subject("session", "b"), linked: { parent: "session:a" } }, subject("session", "j")];
  const listed = listOf({ subjects: all, running: new Set() } as unknown as Ship, deckNow());
  expect(listed.map((one) => one.key).sort()).toEqual(["session:a", "session:b", "session:j"]);
});

test("a linked Session whose parent is not listed stands in the list where it would", () => {
  const orphan: Subject = { ...subject("session", "c"), linked: { parent: "session:gone" } };
  const listed = listOf({ subjects: [subject("session", "j"), orphan], running: new Set() } as unknown as Ship, deckNow());
  expect(listed.map((one) => one.key).sort()).toEqual(["session:c", "session:j"]);
});

test("a linked Session whose parent is not listed is drawn as a plain top-level row, without its link", () => {
  const orphan: Subject = { ...subject("session", "c"), linked: { parent: "session:gone" } };
  const child: Subject = { ...subject("session", "k"), linked: { parent: "session:p" } };
  const listed = listOf({ subjects: [subject("session", "j"), orphan, subject("session", "p"), child], running: new Set() } as unknown as Ship, deckNow());
  expect(listed.find((one) => one.key === "session:c")?.linked).toBeUndefined();
  expect(listed.find((one) => one.key === "session:k")?.linked).toEqual({ parent: "session:p", depth: 1 });
});

// ---- The order he placed ------------------------------------------------------------------------------

const toned = (id: string, tone: "ask" | "calm", phase: Subject["phase"] = "working"): Subject => ({ ...subject("session", id), mark: { Glyph: () => null, said: "", tone }, phase }) as unknown as Subject;
const shipOf = (...subjects: Subject[]): Ship => ({ subjects, running: new Set() }) as unknown as Ship;
const stored = () => JSON.parse(kept.get(DECK_ORDER_KEY) ?? "[]") as string[];
const keys = (rows: readonly Subject[]) => rows.map((one) => one.key);

test("a row whose call is quieted keeps its place in the list", () => {
  placeNew(shipOf(toned("a", "calm"), toned("b", "ask", "needs"), toned("c", "calm")));
  const before = keys(listOf(shipOf(toned("a", "calm"), toned("b", "ask", "needs"), toned("c", "calm")), deckNow()));
  // Opening b quiets it, and the standing order would drop it below the working row.
  const after = keys(listOf(shipOf(toned("a", "calm"), toned("b", "calm", "needs"), toned("c", "calm", "done")), deckNow()));
  expect(after).toEqual(before);
});

test("a Session the order has not met is placed at the bottom, and nothing placed moves", () => {
  placeNew(shipOf(toned("a", "calm"), toned("b", "calm")));
  expect(deckNow().order).toEqual(["session:a", "session:b"]);

  // The new one is loud, and would stand first by standing; it takes the bottom.
  const grown = shipOf(toned("a", "calm"), toned("b", "calm"), toned("new", "ask", "needs"));
  placeNew(grown);
  expect(deckNow().order).toEqual(["session:a", "session:b", "session:new"]);
  expect(keys(listOf(grown, deckNow()))).toEqual(["session:a", "session:b", "session:new"]);

  // Meeting the same Sessions again places nothing and tells nobody.
  const was = deckNow();
  placeNew(grown);
  expect(deckNow()).toBe(was);
});

test("moving a row keeps the new order on this machine, and a new window reads it", () => {
  const ship = shipOf(toned("a", "calm"), toned("b", "calm"), toned("c", "calm"));
  placeNew(ship);
  moveRow(ship, "session:c", "session:a", false);
  expect(deckNow().order).toEqual(["session:c", "session:a", "session:b"]);
  expect(stored()).toEqual(["session:c", "session:a", "session:b"]);

  forgetDeck();
  expect(deckNow().order).toEqual(["session:c", "session:a", "session:b"]);
  expect(keys(listOf(ship, deckNow()))).toEqual(["session:c", "session:a", "session:b"]);

  moveRow(ship, "session:c", "session:b", true);
  expect(stored()).toEqual(["session:a", "session:b", "session:c"]);
});

// ---- A Job handed on to a new one ----------------------------------------------------------------------

const gone = (id: string): Subject => ({ ...subject("session", id), past: "ended", behind: true, at: 1 }) as unknown as Subject;
const took = (id: string, earlier: string): Subject => ({ ...toned(id, "calm"), earlier }) as unknown as Subject;

test("the new Job takes the old Job's place in the row order", () => {
  placeNew(shipOf(toned("a", "calm"), toned("old", "calm"), toned("c", "calm")));
  expect(deckNow().order).toEqual(["session:a", "session:old", "session:c"]);

  const after = shipOf(toned("a", "calm"), took("new", "old"), toned("c", "calm"), gone("old"));
  placeNew(after);
  expect(deckNow().order).toEqual(["session:a", "session:new", "session:c"]);
  expect(keys(listOf(after, deckNow()))).toEqual(["session:a", "session:new", "session:c"]);
});

test("the old Job is hidden behind the new one in every scope, and in the list only its mark reaches it", () => {
  const after = shipOf(took("new", "old"), gone("old"), { ...subject("session", "done"), past: "ended", at: 0 } as unknown as Subject);
  for (const which of ["active", "quiet", "ended", "all"] as const) {
    expect(keys(listOf(after, { ...deckNow(), scope: which }))).not.toContain("session:old");
  }
  expect(keys(listOf(after, { ...deckNow(), scope: "ended" }))).toEqual(["session:done"]);
  // Open on the stage from the earlier mark, it is still not a row.
  expect(keys(listOf(after, { ...deckNow(), scope: "all", open: "session:old" }))).not.toContain("session:old");
});
