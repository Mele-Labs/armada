import { describe, expect, test } from "vitest";

import type { Item } from "../Dashboard";
import type { Bay, BaysRead } from "./bays";
import type { Dot } from "./horizon";
import { MAIN, metroOf, orderOf } from "./metro";

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
      ["Job c", 4],
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

  test("main is the last station, and the keys step down the lines in the order they are drawn", () => {
    const metro = metroOf(read([bay(1, job("a")), bay(2, job("b"), { dot: dot({}) })], [tile("w", { hue: "queued", job: { id: "w" } as Item["job"] })]), NOW);
    expect(MAIN).toBe(5);
    expect(orderOf(metro).map((one) => one.key)).toEqual(["b", "a", "w"]);
  });
});
