// What a ring is read from: a Session's `build` as Fleet carries it, a Job's plan tasks, and the quiet that dims what is unfinished.

import type { JobDetail, SessionBuild, TimelineEvent } from "@armada/protocol";
import { workingAPlan } from "@armada/jobs/fixtures/build/index";
import { expect, test } from "vitest";

import { PHASE_GLYPH, askedOf, seatOf, STALL_MIN, STOP_GLYPH, arcsOf, bandOf, buildInOf, buildTipOf, dimOf, glyphOf, jobBuildOf, jobPhaseOf, markersOf, roadOf, stallOf, wordOf, writingOf } from "./build";

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

test("a ring with no stop and no landing has no road, as a Job's has none", () => {
  expect(roadOf(buildInOf("s1", build({ phase: "implement" }), NOW))).toBeUndefined();
  expect(roadOf(jobBuildOf(jobWith(["working"]), "job:j1", NOW)!)).toBeUndefined();
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
  expect(tip.stops.map((one) => [one.word, one.ago])).toEqual([["Walk shown", "30 min ago"], ["Checks", undefined], ["Landed", undefined]]);
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

const jobWith = (states: string[]): JobDetail => {
  const fixture = workingAPlan();
  if (fixture.watched.state !== "read" || fixture.watched.detail.work_plan === undefined) throw new Error("the fixture holds no plan");
  const detail = fixture.watched.detail;
  return { ...detail, work_plan: { ...detail.work_plan, tasks: states.map((state, i) => ({ id: `T${i}`, title: `Task ${i}`, state })) } } as JobDetail;
};

test("a Job's tasks are its pieces by state: done solid, handed in hollow, working scaffold, open or failed ghost, dropped none", () => {
  const read = jobBuildOf(jobWith(["done", "handed_in", "working", "open", "failed", "dropped"]), "job:j1", NOW);
  expect(read?.tasks.map((one) => [one.text, one.piece, one.said])).toEqual([
    ["Task 0", "solid", "Done"],
    ["Task 1", "hollow", "Handed in"],
    ["Task 2", "scaffold", "Working"],
    ["Task 3", "ghost", "Open"],
    ["Task 4", "ghost", "Failed"],
  ]);
  expect(read?.key).toBe("job:j1");
  expect(read?.stops).toEqual([]);
});

test("a Job with no live task has no ring, and its row has no phase", () => {
  expect(jobBuildOf(jobWith([]), "job:j1", NOW)).toBeUndefined();
  expect(jobBuildOf(jobWith(["dropped"]), "job:j1", NOW)).toBeUndefined();
  expect(jobPhaseOf(undefined)).toBeUndefined();
  expect(jobPhaseOf({ done: 0, working: 0, open: 0, dropped: 2 })).toBeUndefined();
  expect(jobPhaseOf({ done: 1, working: 0, open: 0, dropped: 0 })).toBe("implement");
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
