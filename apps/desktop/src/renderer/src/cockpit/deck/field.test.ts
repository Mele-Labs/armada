// What the battlefield is handed, as pure readings: which drones carry a call marker, how many action loops run, what a callout says
// of a drone besides the drone, what arrives while the field is on screen, and the two things that keep a busy window quiet (the
// clock quantized, and the timeline's appends batched).

import { expect, test } from "vitest";
import type { TimelineEvent } from "@armada/protocol";

import { MOST_LOOPS } from "./doing";
import { arrivalsOf, callMarkersOf, eventFactsOf, filesNamed, loopsOf, pillsOf, quantized, throttled, TICK_MS } from "./field";
import type { DroneIn, FileAct } from "./layout";
import { MOST_PLAYING } from "./plays";

const drone = (key: string, extra: Partial<DroneIn> = {}): DroneIn => ({
  key,
  kind: "session",
  title: key,
  root: "/r",
  said: "Working",
  hue: "calm",
  spin: false,
  hover: false,
  ping: false,
  stuck: false,
  flare: false,
  working: false,
  touches: [],
  shown: true,
  ...extra,
});

const event = (id: number, minutes: number, extra: Partial<TimelineEvent> = {}): TimelineEvent =>
  ({ id, at: new Date(Date.UTC(2026, 9, 14, 12, 0) - minutes * 60_000).toISOString(), kind: "session.turn", subject: { kind: "session", id: "a" }, tone: "run", says: `says ${id}`, ...extra }) as TimelineEvent;
const CLOCK = Date.UTC(2026, 9, 14, 12, 0);
const own = (e: TimelineEvent) => (e.subject.kind === "session" ? `session:${e.subject.id}` : undefined);

// ---- A call marker ------------------------------------------------------------------------------------

test("a drone that holds a call he has not opened carries a marker, amber while it is owed and red while its Check fails", () => {
  const drones = [drone("session:a", { ping: true }), drone("session:b", { ping: true, flare: true }), drone("session:quiet")];
  const owed = (key: string) => (key === "session:quiet" ? undefined : `call:${key}`);
  const marks = callMarkersOf(drones, owed, new Set(), true);

  expect([...marks]).toEqual([["session:a", "ask"], ["session:b", "bad"]]);
});

test("a call he has opened or answered no longer marks its drone, and a drone the filters dim carries none", () => {
  const drones = [drone("session:a", { ping: true }), drone("session:b", { ping: true, shown: false })];
  const owed = (key: string) => `call:${key}`;

  expect(callMarkersOf(drones, owed, new Set(["call:session:a"]), true).size).toBe(0);
  expect(callMarkersOf(drones, (key) => (key === "session:a" ? undefined : owed(key)), new Set(), true).size).toBe(0);
});

test("in a replay the markers are the drones that were calling at the playhead, whatever has since been opened", () => {
  const drones = [drone("session:a", { ping: true }), drone("session:b", { ping: true, flare: true }), drone("session:c")];
  const marks = callMarkersOf(drones, () => undefined, new Set(["x"]), false);

  expect([...marks]).toEqual([["session:a", "ask"], ["session:b", "bad"]]);
});

// ---- The loops ------------------------------------------------------------------------------------------

test("at most eight action animations loop, the focused first, then the selected, then the one under the pointer", () => {
  const doers = Array.from({ length: 14 }, (_, i) => drone(`session:${i}`, { working: true, doing: "read" }));
  const loops = loopsOf(doers, { focus: "session:13", selected: "session:12", hot: "session:11" });

  expect(loops.size).toBe(MOST_LOOPS);
  expect(MOST_LOOPS).toBe(8);
  for (const first of ["session:13", "session:12", "session:11"]) expect(loops.has(first)).toBe(true);
  expect(loopsOf(doers.slice(0, 3), {}).size).toBe(3);
});

// ---- What the callout says besides the drone -------------------------------------------------------------

test("its last event is the latest of its own at or before the clock, and the last hour is a bar for each five minutes", () => {
  const events = [event(1, 50), event(2, 12), event(3, 3), event(4, 2, { subject: { kind: "session", id: "other" } }), event(5, -20)];
  const facts = eventFactsOf(events, "session:a", own, CLOCK);

  expect(facts.last?.says).toBe("says 3");
  expect(facts.ticks).toHaveLength(12);
  // The newest bucket holds the last five minutes, the oldest the hour's first.
  expect(facts.ticks[11]).toBe(1);
  expect(facts.ticks[9]).toBe(1);
  expect(facts.ticks[1]).toBe(1);
  expect(facts.ticks.reduce((sum, n) => sum + n, 0)).toBe(3);
});

test("a drone with no event has no last line", () => {
  expect(eventFactsOf([], "session:a", own, CLOCK).last).toBeUndefined();
});

test("the areas it touches are its two biggest, a long one cut from the left", () => {
  const touched = drone("session:a", { touches: [{ path: "crates/a", w: 1 }, { path: "apps/desktop", w: 9 }, { path: "docs/concepts", w: 4 }] });
  expect(pillsOf(touched)).toEqual(["apps/desktop", "docs/concepts"]);
  expect(pillsOf(drone("session:b"))).toEqual([]);
  const long = drone("session:c", { touches: [{ path: "packages/surfaces/jobs/src/draft/long", w: 1 }] });
  expect(pillsOf(long)[0]!.startsWith("…")).toBe(true);
  expect(pillsOf(long)[0]!.length).toBeLessThanOrEqual(26);
});

// ---- What arrives while the field is drawn ---------------------------------------------------------------

test("a drone that was not there lifts from its base, and a pull request that has landed since is captured, each once", () => {
  const was = new Set(["session:a"]);
  const out = arrivalsOf({ was, landedWas: new Set(["5"]), drones: ["session:a", "job:new"], landed: new Map([[5, "/r"], [6, "/r"]]), seen: new Set() });

  expect(out.map((one) => [one.id, one.kind])).toEqual([["arrived:job:new", "launch"], ["merged:6", "capture"]]);
  expect(arrivalsOf({ was, landedWas: new Set(["5"]), drones: ["session:a", "job:new"], landed: new Map([[5, "/r"], [6, "/r"]]), seen: new Set(["arrived:job:new", "merged:6"]) })).toEqual([]);
});

test("the first draw of the field plays nothing as arriving", () => {
  expect(arrivalsOf({ was: undefined, landedWas: undefined, drones: ["session:a"], landed: new Map([[5, "/r"]]), seen: new Set() })).toEqual([]);
});

test("no more than a dozen effects are played at once", () => {
  expect(MOST_PLAYING).toBe(12);
  const drones = Array.from({ length: 30 }, (_, i) => `job:${i}`);
  expect(arrivalsOf({ was: new Set(), landedWas: new Set(), drones, landed: new Map(), seen: new Set() }).length).toBeLessThanOrEqual(MOST_PLAYING);
});

// ---- A busy window stays quiet ----------------------------------------------------------------------------

test("the clock the field reads is quantized to five seconds", () => {
  expect(quantized(10_001)).toBe(10_000);
  expect(quantized(14_999)).toBe(10_000);
  expect(quantized(15_000)).toBe(15_000);
});

test("timeline appends reach the field at no more than four a second: a burst is one delivery of the latest, and the last word is never lost", () => {
  let now = 0;
  const later: { at: number; run: () => void }[] = [];
  const got: number[] = [];
  const hold = throttled<number>((value) => got.push(value), TICK_MS, { now: () => now, later: (run, ms) => void later.push({ at: now + ms, run }) });
  expect(TICK_MS).toBeGreaterThanOrEqual(250);

  hold.push(1);
  hold.push(2);
  hold.push(3);
  // The first is delivered at once; the rest wait for the tick, and only the latest of them is delivered.
  expect(got).toEqual([1]);
  now = TICK_MS;
  later.splice(0).forEach((one) => one.run());
  expect(got).toEqual([1, 3]);

  now += 10;
  hold.push(4);
  expect(got).toEqual([1, 3]);
  now += TICK_MS;
  later.splice(0).forEach((one) => one.run());
  expect(got).toEqual([1, 3, 4]);

  // Quiet for a while: the next one is delivered at once again.
  now += 10 * TICK_MS;
  hold.push(5);
  expect(got).toEqual([1, 3, 4, 5]);
});

// ---- The files Sessions name -------------------------------------------------------------------------

test("where two Sessions name one file the stronger act stands: removed over written, written over edited, edited over at rest", () => {
  const act = (...acts: (FileAct | undefined)[]) => filesNamed(acts.map((one) => ({ root: "/r", file: one === undefined ? { path: "a.rs" } : { path: "a.rs", act: one } }))).files.get("/r");
  expect(act("edit", "write")).toEqual([{ path: "a.rs", act: "write" }]);
  expect(act("write", "edit")).toEqual([{ path: "a.rs", act: "write" }]);
  expect(act("write", "remove", "edit")).toEqual([{ path: "a.rs", act: "remove" }]);
  expect(act(undefined, "edit")).toEqual([{ path: "a.rs", act: "edit" }]);
  expect(act("edit", undefined)).toEqual([{ path: "a.rs", act: "edit" }]);
  expect(act(undefined, undefined)).toEqual([{ path: "a.rs" }]);
});

test("the files are kept by repository and sorted by path, and the key says what they hold", () => {
  const one = filesNamed([{ root: "/b", file: { path: "z.rs" } }, { root: "/a", file: { path: "y.rs", act: "edit" } }, { root: "/a", file: { path: "b.rs" } }]);
  expect([...one.files.keys()]).toEqual(["/b", "/a"]);
  expect(one.files.get("/a")).toEqual([{ path: "b.rs" }, { path: "y.rs", act: "edit" }]);
  // The same files in another order are the same key; another act is not.
  const again = filesNamed([{ root: "/a", file: { path: "b.rs" } }, { root: "/a", file: { path: "y.rs", act: "edit" } }, { root: "/b", file: { path: "z.rs" } }]);
  expect(again.key).toBe(one.key);
  expect(filesNamed([{ root: "/a", file: { path: "b.rs" } }, { root: "/a", file: { path: "y.rs", act: "write" } }, { root: "/b", file: { path: "z.rs" } }]).key).not.toBe(one.key);
});
