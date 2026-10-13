import { describe, expect, test } from "vitest";

import type { Item } from "../Dashboard";
import type { Bay, BaysRead, PullHolder, WorkTrail } from "./bays";
import type { Dot } from "./horizon";
import { MAIN, REVIEW, metroOf, orderOf } from "./metro";

const NOW = Date.parse("2026-10-11T12:00:00Z");
const tile = (key: string, more: Partial<Item> = {}) => ({ key, title: key, hue: "running", fact: "Editing a file", ...more }) as unknown as Item;
const dot = (more: Partial<Dot> & { place?: number }): Dot =>
  ({
    key: "pull:1",
    state: "running",
    queued: false,
    icon: null,
    tip: "",
    act: { kind: "link", url: "https://git.example/pull/1" },
    ...more,
    card: { heading: "#1", title: "First", branch: "feat/a", says: "ci running", check: undefined, place: more.place, owner: undefined },
  }) as Dot;
const bay = (slot: number, holder: Bay["holder"], more: Partial<Bay> = {}): Bay => ({ key: `armada:${slot}`, slot, branch: `feat/${slot}`, warm: true, behind: 0, since: "2026-10-11T11:19:00Z", holder, riders: [], dot: undefined, ...more });
const job = (id: string, more: Partial<Item> = {}) => ({ kind: "job", id, title: `Job ${id}`, item: tile(id, { job: { id } as Item["job"], ...more }) }) as const;
const read = (bays: Bay[], waiting: Item[] = []): BaysRead => ({ harbours: [{ manifest: "armada", name: "armada", bays, line: [], main: "green" }], waiting });

describe("the metro lines", () => {
  test("work in a bay is building until it has a pull request, then at the pull request, then in the queue", () => {
    const metro = metroOf(
      read([bay(1, job("a")), bay(2, job("b"), { dot: dot({ state: "failing" }) }), bay(3, job("c"), { dot: dot({ queued: true, place: 2, state: "queued" }) })]),
      NOW,
    );
    expect(metro.harbours[0]!.lines.map((line) => [line.title, line.reach])).toEqual([
      ["Job c", 5],
      ["Job b", 3],
      ["Job a", 2],
    ]);
  });

  test("the stop it is at now carries its queue place, ! for failing checks, ? when it asks", () => {
    const metro = metroOf(
      read([bay(1, job("q"), { dot: dot({ queued: true, place: 1 }) }), bay(2, job("f"), { dot: dot({ state: "failing" }) }), bay(3, job("x", { hue: "ask" }))]),
      NOW,
    );
    const glyph = (title: string) => {
      const line = metro.harbours[0]!.lines.find((one) => one.title === title)!;
      return line.stops[line.reach]?.glyph;
    };
    expect([glyph("Job q"), glyph("Job f"), glyph("Job x")]).toEqual(["1", "!", "?"]);
  });

  test("each stop takes the state at its station, and the line the work's own", () => {
    const line = metroOf(read([bay(1, job("a", { hue: "ask" }), { dot: dot({ state: "failing" }) })]), NOW).harbours[0]!.lines[0]!;
    expect(line.tone).toBe("ask");
    expect(line.stops[3]?.tone).toBe("bad");
    expect(line.wants).toBe("fixing");
  });

  test("the building stop's ring fills with the Job's steps", () => {
    const steps = [1, 2, 3, 4].map((n) => ({ id: `s${n}`, label: `Step ${n}` }));
    const line = metroOf(read([bay(1, job("a", { steps, stepAt: 1 }))]), NOW).harbours[0]!.lines[0]!;
    expect(line.stops[2]?.progress).toBe(50);
    expect(line.stops[2]?.card.title).toBe("Step 2, step 2 of 4");
  });

  test("a free or closed bay draws no line, but is counted in the strip", () => {
    const metro = metroOf(read([bay(1, job("a")), bay(2, { kind: "free" }), bay(3, { kind: "closed" })]), NOW);
    expect(metro.harbours[0]!.lines).toHaveLength(1);
    expect(metro.bays).toMatchObject({ held: 1, free: 1, closed: 1, strip: ["held", "free", "closed"], names: ["Bay 01: Job a", "Bay 02: free", "Bay 03: closed"] });
  });

  test("work waiting for a bay is a line at Waiting, told which free bay it goes to", () => {
    const waiting = tile("w", { hue: "queued", job: { id: "w", owner_manifest_id: "armada" } as Item["job"] });
    const metro = metroOf(read([bay(1, job("a")), bay(2, { kind: "free" })], [waiting]), NOW);
    expect(metro.waiting[0]!.reach).toBe(0);
    expect(metro.waiting[0]!.stops[0]?.card.rows).toEqual([["Goes to", "bay 02, the first free one"]]);
  });

  test("a stop's card opens what it is about: the Job, and the pull request on the forge", () => {
    const line = metroOf(read([bay(1, job("a"), { dot: dot({}) })]), NOW).harbours[0]!.lines[0]!;
    expect(line.stops[2]?.card.acts).toEqual([{ kind: "job", id: "a", label: "Open Job" }]);
    expect(line.stops[3]?.card.acts).toEqual([{ kind: "link", url: "https://git.example/pull/1", label: "Open #1" }]);
    expect(line.stops[1]?.card.rows).toContainEqual(["Held", "41m"]);
  });

  test("a line's title opens its Job, or its Session, and nothing for a terminal Armada does not know", () => {
    const known = { kind: "session", title: "fix/x", session: { id: "s1" }, item: undefined, by: undefined } as unknown as Bay["holder"];
    const stranger = { kind: "session", title: "fix/y", session: undefined, item: undefined, by: "A terminal, pid 9" } as unknown as Bay["holder"];
    const lines = metroOf(read([bay(1, job("a")), bay(2, known), bay(3, stranger)]), NOW).harbours[0]!.lines;
    expect(lines.map((line) => line.open)).toEqual([{ kind: "job", id: "a", label: "Open Job" }, { kind: "session", id: "s1", label: "Open Session" }, undefined]);
    expect(lines[2]!.stops[1]?.card.rows).toContainEqual(["Held by", "A terminal, pid 9"]);
  });

  test("a pull request no bay holds is a line of its own, from Pull request on, titled by its branch and opening on the forge", () => {
    const inBay = dot({ key: "pull:1" });
    const loose = { ...dot({ key: "pull:7", queued: true, place: 1, state: "queued" }), card: { ...dot({}).card, heading: "#7", branch: "feat/by-hand", place: 1, title: "Fix by hand" } } as Dot;
    const open = { ...dot({ key: "pull:8", state: "failing" }), card: { ...dot({}).card, heading: "#8", branch: "check/other" } } as Dot;
    const harbour = { manifest: "armada", name: "armada", bays: [bay(1, job("a"), { dot: inBay })], line: [inBay, open, loose], main: "green" as const };
    const outside = metroOf({ harbours: [harbour], waiting: [] }, NOW).harbours[0]!.outside;
    expect(outside.map((line) => [line.title, line.from, line.reach, line.stops[line.reach]?.glyph])).toEqual([
      ["feat/by-hand", 3, 5, "1"],
      ["check/other", 3, 3, "!"],
    ]);
    expect(outside[0]!.stops.slice(0, 3)).toEqual([undefined, undefined, undefined]);
    expect(outside[0]!.stops[3]?.card.rows).toContainEqual(["Title", "Fix by hand"]);
    expect(outside[0]!.open).toEqual({ kind: "link", url: "https://git.example/pull/1", label: "Open #7" });
  });

  test("main is the last station, and the keys step down the lines in the order they are drawn", () => {
    const metro = metroOf(read([bay(1, job("a")), bay(2, job("b"), { dot: dot({}) })], [tile("w", { hue: "queued", job: { id: "w" } as Item["job"] })]), NOW);
    expect(MAIN).toBe(6);
    expect(orderOf(metro).map((one) => one.key)).toEqual(["b", "a", "w"]);
  });
});

/** One pull request outside every bay, with the facts the forge names and the owner's trail where it has one. */
const outsidePull = (facts: Partial<Dot["card"]> & { state?: Dot["state"]; queued?: boolean; place?: number }, trail?: WorkTrail) => {
  const { state, queued, place, ...card } = facts;
  const pull = { ...dot({ key: "pull:9", state: state ?? "open", queued: queued ?? false, place }), card: { ...dot({}).card, heading: "#9", branch: "feat/nine", title: "Nine", place, ...card } } as Dot;
  const holders = new Map<string, PullHolder>(trail === undefined ? [] : [["pull:9", { kind: "session", id: "s1", title: "Nine", item: undefined, trail }]]);
  const metro = metroOf({ harbours: [{ manifest: "armada", name: "armada", bays: [], line: [pull], holders, main: "green" }], waiting: [] }, NOW);
  return { metro, line: metro.harbours[0]!.outside[0]! };
};

describe("the trail behind a pull request", () => {
  const ago = (minutes: number) => new Date(NOW - minutes * 60_000).toISOString();

  test("a Session's trail draws waited, the bay it was in and built as passed stations, the line starting at the first", () => {
    const { line } = outsidePull({}, { began: ago(75), bays: [{ slot: 10, from: ago(70), until: ago(20) }], opened: ago(30) });
    expect(line.from).toBe(0);
    expect(line.session).toBe(true);
    expect(line.stops.slice(0, 3).map((stop) => [stop?.tone, stop?.card.pill, stop?.card.title])).toEqual([
      ["ok", "waited", "Waited for a bay"],
      ["ok", "bay 10", "Was in bay 10"],
      ["ok", "built", "Built"],
    ]);
    expect(line.stops[0]?.card.rows).toEqual([["Started", "1h 15m ago"], ["Waited", "5m"]]);
    expect(line.stops[1]?.card.rows).toEqual([["Took it", "1h 10m ago"], ["Gave it back", "20m ago"], ["Held", "50m"]]);
    expect(line.stops[2]?.card.rows).toContainEqual(["Pull request opened", "30m ago"]);
    expect(line.stops[3]?.card.rows).toContainEqual(["Opened", "30m ago"]);
  });

  test("a Job's bay history tells each stay, the last one in the pill", () => {
    const { line } = outsidePull({}, { began: ago(200), bays: [{ slot: 2, from: ago(190), until: ago(120) }, { slot: 5, from: ago(100) }] });
    expect(line.stops[1]?.card).toMatchObject({ pill: "bay 05", title: "Was in bays 02, 05" });
    expect(line.stops[1]?.card.rows).toEqual([["Bay 02", "3h 10m ago, held 1h 10m"], ["Bay 05", "1h 40m ago"]]);
    expect(line.stops[0]?.card.rows).toContainEqual(["Waited", "10m"]);
  });

  test("a trail that names no bay is a Job that started, so only the first stations are told", () => {
    const { line } = outsidePull({}, { began: ago(60), bays: [{ from: ago(50) }] });
    expect(line.stops[1]?.card).toMatchObject({ pill: "in", title: "Was in a bay" });
  });

  test("a pull request nobody owns takes its opened time from the forge, and starts at Pull request", () => {
    const { line } = outsidePull({ opened: ago(12) });
    expect(line.from).toBe(3);
    expect(line.stops[3]?.card.rows).toContainEqual(["Opened", "12m ago"]);
  });
});

describe("the review station", () => {
  test("approved is passed and not yet in the queue, so the line is at Review", () => {
    const { line, metro } = outsidePull({ review: "approved" });
    expect(line.stops[REVIEW]).toMatchObject({ tone: "ok", card: { pill: "approved", title: "Review approved" } });
    expect(line.reach).toBe(REVIEW);
    expect(metro.review).toBe(true);
  });

  test("changes requested takes the failed hue and a !, review required waits", () => {
    expect(outsidePull({ review: "changes" }).line.stops[REVIEW]).toMatchObject({ tone: "bad", glyph: "!", card: { title: "Changes requested" } });
    expect(outsidePull({ review: "required" }).line.stops[REVIEW]).toMatchObject({ tone: "idle", card: { pill: "required", title: "Review required" } });
  });

  test("it sits between the pull request and the merge queue, which stays the line's reach once the pull request is in it", () => {
    const { line } = outsidePull({ review: "approved", state: "queued", queued: true, place: 2 });
    expect(line.reach).toBe(5);
    expect(line.stops.map((stop) => stop?.card.title)).toEqual([undefined, undefined, undefined, "Pull request #9", "Review approved", "2nd in the merge queue", undefined]);
  });

  test("with no review known anywhere there is no station to draw, and the line is where it was", () => {
    const { line, metro } = outsidePull({});
    expect(line.stops[REVIEW]).toBeUndefined();
    expect(line.reach).toBe(3);
    expect(metro.review).toBe(false);
  });

  test("a pull request in a bay has one too", () => {
    const reviewed = { ...dot({}), card: { ...dot({}).card, review: "approved" as const } };
    const line = metroOf(read([bay(1, job("a"), { dot: reviewed })]), NOW).harbours[0]!.lines[0]!;
    expect(line.stops[REVIEW]?.card.title).toBe("Review approved");
    expect(line.reach).toBe(REVIEW);
  });
});

describe("a draft or conflicting pull request", () => {
  test("a draft is a dashed station that says Draft", () => {
    const stop = outsidePull({ draft: true }).line.stops[3]!;
    expect(stop.dashed).toBe(true);
    expect(stop.card).toMatchObject({ pill: "Draft", title: "Draft pull request #9" });
    expect(stop.card.rows).toContainEqual(["Status", "ci running"]);
  });

  test("a conflict takes the failed hue and says it conflicts with main, whatever else it is", () => {
    const stop = outsidePull({ conflicts: true, draft: true }).line.stops[3]!;
    expect(stop).toMatchObject({ tone: "bad", glyph: "!", dashed: true, card: { tone: "bad", pill: "Conflicts with main" } });
  });

  test("an ordinary one is neither dashed nor renamed", () => {
    const stop = outsidePull({}).line.stops[3]!;
    expect(stop.dashed).toBeUndefined();
    expect(stop.card.pill).toBe("ci running");
    expect(stop.card.rows.some(([label]) => label === "Status")).toBe(false);
  });
});
