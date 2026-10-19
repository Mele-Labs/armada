// What a ring is read from: a Session's `build` as Fleet carries it, and the quiet that dims what is unfinished.

import type { SessionBuild, TimelineEvent } from "@armada/protocol";
import { expect, test } from "vitest";

import { PHASE_GLYPH, askedOf, groupedTasks, seatOf, seatedWorkers, STALL_MIN, STOP_GLYPH, arcsOf, bandOf, buildInOf, buildTipOf, dimOf, glyphOf, markersOf, roadOf, stallOf, wordOf, writingOf } from "./build";

const NOW = Date.parse("2026-10-14T12:00:00Z");
const ago = (minutes: number) => new Date(NOW - minutes * 60_000).toISOString();

const build = (change: Partial<SessionBuild> = {}): SessionBuild => ({ phase: "implement", pieces: [], stops: [], forecast: [], stalled: false, workers: [], ...change });

let count = 0;
const event = (kind: TimelineEvent["kind"], id: string, minutes: number): TimelineEvent => {
  count += 1;
  return { id: count, at: ago(minutes), kind, subject: { kind: "session", id }, tone: "run", says: kind };
};

test("a Session's pieces, workers and landing are the record's, and its key is the drone's", () => {
  const read = buildInOf("s1", build({ pieces: [{ id: "a", text: "Dock layout", piece: "solid" }, { id: "b", text: "Dock store", piece: "scaffold" }], workers: ["Explore: map the call sites"], landed_at: ago(3) }), NOW);
  expect(read.key).toBe("session:s1");
  expect(read.tasks).toEqual([{ text: "Dock layout", piece: "solid" }, { text: "Dock store", piece: "scaffold" }]);
  expect(read.workers).toEqual(["Explore: map the call sites"]);
  expect(read.landedAt).toBe(NOW - 3 * 60_000);
});

test("a stop with no instant is a forecast, and the stop last reached is the latest instant", () => {
  const read = buildInOf("s1", build({ phase: "road", stops: [{ id: "walk", at: ago(30) }, { id: "pr", at: ago(10) }, { id: "pushed", at: ago(20) }, { id: "checks" }, { id: "auto_merge" }] }), NOW);
  expect(read.stops.map((one) => [one.id, one.at === undefined])).toEqual([["walk", false], ["pr", false], ["pushed", false], ["checks", true], ["auto_merge", true]]);
  expect(read.reached).toBe("pr");
  expect(wordOf(read)).toBe("Pull request opened");
  expect(glyphOf(read)).toBe(STOP_GLYPH.pr);
});

test("the road's arcs are the stops in order with the landing closing them", () => {
  const read = buildInOf("s1", build({ phase: "road", stops: [{ id: "walk", at: ago(5) }, { id: "checks" }] }), NOW);
  expect(arcsOf(read).map((one) => [one.id, one.at === undefined])).toEqual([["walk", false], ["checks", true], ["landed", true]]);
});

test("before the road is laid the phase is its own word and glyph, and with no stop the card lists none", () => {
  const read = buildInOf("s1", build({ phase: "plan" }), NOW);
  expect(wordOf(read)).toBe("Plan");
  expect(glyphOf(read)).toBe(PHASE_GLYPH.plan);
  expect(buildTipOf(read).stops).toEqual([]);
});

test("the road is drawn in any phase once it has a stop, lit where reached and faint where it is a forecast, the landing closing it", () => {
  const record = build({ phase: "scope", stops: [{ id: "pr", at: ago(8) }, { id: "checks", at: ago(4) }, { id: "auto_merge" }] });
  const read = buildInOf("s1", record, NOW);
  expect(roadOf(read)?.map((one) => [one.id, one.at !== undefined])).toEqual([["pr", true], ["checks", true], ["auto_merge", false], ["landed", false]]);
  expect(buildTipOf(read).stops.map((one) => one.word)).toEqual(["Pull request opened", "Checks", "Auto-merge", "Landed"]);
  for (const phase of ["scope", "plan", "implement", "road", "landed"] as const) expect(roadOf(buildInOf("s1", { ...record, phase }, NOW))).toHaveLength(4);
});

test("a ring with no stop and no landing has no road", () => {
  expect(roadOf(buildInOf("s1", build({ phase: "implement" }), NOW))).toBeUndefined();
  expect(roadOf(buildInOf("s1", build({ phase: "landed", landed_at: ago(1) }), NOW))).toHaveLength(1);
});

test("with no task list the ring is one band: outline while scoping, scaffold once edits are seen, solid when landed, never part filled", () => {
  const band = (change: Partial<SessionBuild>, events: TimelineEvent[] = []) => bandOf(buildInOf("s1", build(change), NOW, events));
  expect(band({ phase: "scope" })).toEqual({ state: "outline", moving: false, word: "Scope" });
  expect(band({ phase: "plan" })?.state).toBe("outline");
  expect(band({ phase: "implement" })).toEqual({ state: "scaffold", moving: false, word: "Implement" });
  expect(band({ phase: "road" })?.state).toBe("scaffold");
  expect(band({ phase: "landed", landed_at: ago(1) })).toEqual({ state: "solid", moving: false, word: "Landed" });
  expect(band({ phase: "implement", pieces: [{ id: "a", text: "Dock", piece: "ghost" }] })).toBeUndefined();
});

test("the band turns only while the drone is writing: a fresh edit, and not stalled", () => {
  const moving = (change: Partial<SessionBuild>, events: TimelineEvent[]) => bandOf(buildInOf("s1", build({ phase: "implement", ...change }), NOW, events))?.moving;
  expect(moving({}, [event("session.edited", "s1", 1)])).toBe(true);
  expect(moving({}, [event("session.edited", "s1", 1), event("session.ran", "s1", 0.5)])).toBe(false);
  expect(moving({}, [event("session.edited", "s1", 5)])).toBe(false);
  expect(moving({}, [event("session.edited", "other", 1)])).toBe(false);
  expect(moving({ stalled: true, quiet_since: ago(20) }, [event("session.edited", "s1", 1)])).toBe(false);
  expect(writingOf([], "s1", NOW)).toBe(false);
});

test("once the road is laid the card lists each stop, with when it happened or none, and the landing last", () => {
  const tip = buildTipOf(buildInOf("s1", build({ phase: "road", stops: [{ id: "walk", at: ago(30) }, { id: "checks" }] }), NOW));
  expect(tip.stops.map((one) => [one.word, one.ago])).toEqual([["Walk shown", "30m ago"], ["Checks", undefined], ["Landed", undefined]]);
});

test("survey markers are the walks shown and questions asked by that Session, while it is being scoped", () => {
  const events = [event("session.asked", "s1", 20), event("session.walked", "s1", 30), event("session.walked", "other", 25), event("session.read", "s1", 10)];
  expect(markersOf(events, "s1")).toEqual(["walk", "asked"]);
  expect(buildInOf("s1", build({ phase: "scope" }), NOW, events).markers).toEqual(["walk", "asked"]);
  expect(buildInOf("s1", build({ phase: "implement" }), NOW, events).markers).toEqual([]);
});

test("nothing dims before the stall, 0.3 at it and 0.65 at forty minutes, and no further", () => {
  expect(dimOf(STALL_MIN - 1)).toBe(0);
  expect(dimOf(STALL_MIN)).toBeCloseTo(0.3);
  expect(dimOf(25)).toBeCloseTo(0.475);
  expect(dimOf(40)).toBeCloseTo(0.65);
  expect(dimOf(600)).toBeCloseTo(0.65);
});

test("a Session dims by how long ago Fleet last saw it work, only while it is stalled", () => {
  expect(stallOf({ stalled: false, quiet_since: ago(90) }, NOW)).toEqual({ stalled: false, quiet: 0, dim: 0 });
  const forty = stallOf({ stalled: true, quiet_since: ago(40) }, NOW);
  expect(forty.quiet).toBe(40);
  expect(forty.dim).toBeCloseTo(0.65);
});

test("a stall is never read as younger than the stall itself, and with no instant held it is as old as the stall", () => {
  expect(stallOf({ stalled: true, quiet_since: ago(2) }, NOW).quiet).toBe(STALL_MIN);
  expect(stallOf({ stalled: true }, NOW)).toEqual({ stalled: true, quiet: STALL_MIN, dim: dimOf(STALL_MIN) });
});

test("a piece a subagent wrote carries its worker's label, kept when the subagent has ended, and one the Session wrote carries none", () => {
  const read = buildInOf("s1", build({ pieces: [{ id: "a", text: "Read", piece: "solid" }, { id: "b", text: "Find call sites", piece: "scaffold", by: "Explore: map the call sites" }], workers: [] }), NOW);
  expect(read.tasks).toEqual([{ text: "Read", piece: "solid" }, { text: "Find call sites", piece: "scaffold", by: "Explore: map the call sites" }]);
  expect(askedOf("Explore: map the call sites")).toBe("map the call sites");
});

test("a worker sits at the middle of the pieces it wrote, from the top and clockwise, and nowhere where it wrote none", () => {
  const tasks = [{}, { by: "Explore: a" }, { by: "Explore: a" }, { by: "Plan: b" }];
  expect(seatOf(tasks, "Explore: a")).toBe(-90 + (1.5 * 360) / 4 + 0 + ((2.5 - 1.5) * 360) / 8);
  expect(seatOf(tasks, "Plan: b")).toBe(-90 + (3.5 * 360) / 4);
  expect(seatOf(tasks, "Explore: c")).toBeUndefined();
});

test("parallel subagents' tasks are drawn as one stretch each, in the order they started, each where its first task stood", () => {
  const t = (text: string, by?: string) => ({ text, ...(by === undefined ? {} : { by }) });
  const list = [t("m1"), t("a1", "A"), t("b1", "B"), t("m2"), t("a2", "A"), t("b2", "B"), t("c1", "C"), t("m3"), t("a3", "A")];
  expect(groupedTasks(list).map((one) => one.text)).toEqual(["m1", "a1", "a2", "a3", "b1", "b2", "m2", "c1", "m3"]);
  expect(groupedTasks(list).filter((one) => one.by === undefined).map((one) => one.text)).toEqual(["m1", "m2", "m3"]);
  expect(groupedTasks([t("x"), t("y")]).map((one) => one.text)).toEqual(["x", "y"]);
});

test("a Session's ring is grouped, and a finished subagent's stretch stays while its worker is gone", () => {
  const piece = (id: string, by?: string) => ({ id, text: id, piece: "solid" as const, ...(by === undefined ? {} : { by }) });
  const read = buildInOf("s1", build({ pieces: [piece("a1", "Explore: a"), piece("b1", "Plan: b"), piece("a2", "Explore: a")], workers: ["Plan: b"] }), NOW);
  expect(read.tasks.map((one) => one.text)).toEqual(["a1", "a2", "b1"]);
  expect(seatOf(read.tasks, "Plan: b")).toBe(-90 + (2.5 * 360) / 3);
});

test("ten running subagents are seated beside their own stretches and no two drones overlap", () => {
  const labels = Array.from({ length: 10 }, (_, i) => `Explore: ${i}`);
  const tasks = groupedTasks(Array.from({ length: 20 }, (_, k) => ({ by: labels[k % 10]! })));
  const seats = seatedWorkers(tasks, labels);
  expect(seats).toHaveLength(10);
  const at = (s: (typeof seats)[number]) => [s.radius * Math.cos((s.angle * Math.PI) / 180), s.radius * Math.sin((s.angle * Math.PI) / 180)] as const;
  for (const a of seats) for (const b of seats) if (a !== b) expect(Math.hypot(at(a)[0] - at(b)[0], at(a)[1] - at(b)[1])).toBeGreaterThanOrEqual(9);
  // Uneven stretches crowd the outer tier, so some go a step inward and still do not overlap.
  const crowded = [...Array.from({ length: 3 }, (_, i) => ({ by: `w${i}` })), ...Array.from({ length: 15 }, () => ({}))];
  const near = seatedWorkers(groupedTasks(crowded), Array.from({ length: 3 }, (_, i) => `w${i}`));
  expect(new Set(near.map((one) => one.radius)).size).toBeGreaterThan(1);
  for (const a of near) for (const b of near) if (a !== b) expect(Math.hypot(at(a)[0] - at(b)[0], at(a)[1] - at(b)[1])).toBeGreaterThanOrEqual(9);
});
