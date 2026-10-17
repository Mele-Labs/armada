// The deck's store: the ways of reading that outlive a restart are remembered, a window with nothing stored opens plain,
// and the calls still to be opened come newest first by the Item's own arrival.

import { afterEach, beforeEach, expect, test, vi } from "vitest";

import type { Item } from "../../Dashboard";
import { shipOf, type Ship, type Subject } from "./ship";
import { arrivedOf, deckNow, forgetDeck, incomingOf, keepOf, needingOf, listOf, moveCursor, setJobView, setScope, setView, toggleChecks, toggleNeeds, toggleOnly, withoutOpenSession } from "./store";

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

test("a Session or Job raises one call however much it asks, the newest standing for the rest", () => {
  const job = (key: string, at: string, id: string): Item => ({ key, at, job: { id } }) as Item;
  const calls = [
    call("session:s1:q1", "2026-10-14T09:00:00Z"),
    call("session:s1:q2", "2026-10-14T11:00:00Z"),
    call("session:s1:w1", "2026-10-14T10:00:00Z"),
    call("session:s2:q1", "2026-10-14T08:00:00Z"),
    job("gate:j1", "2026-10-14T07:00:00Z", "j1"),
    job("check:j1", "2026-10-14T07:30:00Z", "j1"),
    call("main:/r", "2026-10-14T06:00:00Z"),
  ];

  expect(incomingOf(calls, new Map(), new Set()).map((one) => one.key)).toEqual(["session:s1:q2", "session:s2:q1", "check:j1", "main:/r"]);
  // An answered newest gives the Session's next call its place.
  expect(incomingOf(calls, new Map([["session:s1:q2", 1]]), new Set()).map((one) => one.key)[0]).toBe("session:s1:w1");
});

test("a seen call stays while its Session still needs the person, and a seen one from elsewhere leaves", () => {
  const calls = [call("session:s1:q1", "2026-10-14T09:00:00Z"), call("main:/r", "2026-10-14T10:00:00Z")];
  const seen = new Set(["session:s1:q1", "main:/r"]);
  expect(incomingOf(calls, new Map(), seen, new Set(["session:s1"])).map((one) => one.key)).toEqual(["session:s1:q1"]);
  expect(incomingOf(calls, new Map(), seen, new Set())).toEqual([]);
});

test("a Job that needs the person and raised no call of its own is one, and the Calls filter lists it", () => {
  const job = { id: "j1", title: "Review me", status: "awaiting_review", owner_manifest_id: "m" } as never;
  const ship = shipOf({ jobs: [job], sessions: [], lines: [], slots: [], repositories: [], workflows: [], nows: undefined, calls: [], now: 0 });
  const raised = ship.calls.map((one) => one.key);
  expect(raised).toEqual(["job:j1:needs:awaiting_review"]);
  expect(incomingOf(ship.calls, new Map(), new Set(raised), needingOf(ship)).map((one) => one.key)).toEqual(["job:j1:needs:awaiting_review"]);
  toggleNeeds();
  expect(listOf(ship, deckNow()).map((one) => one.key)).toEqual(["job:j1"]);
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

test("moving the cursor to the row it is on tells nobody", () => {
  moveCursor("job:a");
  const before = deckNow();
  moveCursor("job:a");
  expect(deckNow()).toBe(before);
  moveCursor("job:b");
  expect(deckNow().cursor).toBe("job:b");
});

test("the open Session's own calls are cards in its thread and leave Incoming calls, the others' stay", () => {
  const calls = [call("session:s1:q1", "2026-10-14T09:00:00Z"), call("session:s2:q1", "2026-10-14T10:00:00Z"), call("main:/r", "2026-10-14T11:00:00Z")];
  const keys = (sel: Parameters<typeof withoutOpenSession>[1]) => withoutOpenSession(calls, sel).map((one) => one.key);
  expect(keys({ kind: "session", key: "session:s1" })).toEqual(["session:s2:q1", "main:/r"]);
  expect(keys(undefined)).toEqual(["session:s1:q1", "session:s2:q1", "main:/r"]);
  // Only an open Session takes its calls: a Job open on the stage leaves every call where it was.
  expect(keys({ kind: "job", key: "session:s1" })).toHaveLength(3);
});

// ---- Scope and search ----------------------------------------------------------------------------------

const past = (id: string, how: "quiet" | "ended", at: number, older = false): Subject => ({ ...subject("session", id), past: how, at, ...(older ? { older: true } : {}) }) as Subject;
const scoped = (): Ship =>
  ({
    subjects: [subject("job", "live-job"), subject("session", "live"), past("quiet-new", "quiet", 300), past("ended-old", "ended", 100), past("ended-new", "ended", 200), past("ended-ancient", "ended", 1, true)],
    running: new Set(),
    mains: [],
    prs: [],
    calls: [],
  }) as unknown as Ship;
const scope = (which: "active" | "quiet" | "ended" | "all", rest: { found?: ReadonlyMap<string, unknown>; open?: string } = {}) => listOf(scoped(), { ...deckNow(), scope: which, ...rest }).map((one) => one.id);

test("Active lists the live Jobs and Sessions and no past one", () => {
  expect(scope("active")).not.toContain("quiet-new");
  expect(scope("active").sort()).toEqual(["live", "live-job"]);
});

test("Quiet and Ended list their own Sessions, newest first, and an ended one past the week is left off", () => {
  expect(scope("quiet")).toEqual(["quiet-new"]);
  expect(scope("ended")).toEqual(["ended-new", "ended-old"]);
});

test("All lists the live ones and then every past one, newest first", () => {
  const all = scope("all");
  expect(all.slice(0, 2).sort()).toEqual(["live", "live-job"]);
  expect(all.slice(2)).toEqual(["quiet-new", "ended-new", "ended-old"]);
});

test("a search narrows the list to what it found, and it is the only way to a Session past the week", () => {
  const found = new Map([["session:ended-ancient", {}], ["session:live", {}]]);
  expect(scope("active", { found })).toEqual(["live"]);
  expect(scope("ended", { found })).toEqual(["ended-ancient"]);
  expect(scope("all", { found }).sort()).toEqual(["ended-ancient", "live"]);
});

test("the Session open on the stage stays in the list, whatever the scope", () => {
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
