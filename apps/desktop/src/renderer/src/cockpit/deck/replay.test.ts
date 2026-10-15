// A replay is offered only from what the events Bridge holds say: the last two hours and at most 2000 of them, at their
// real times. Before the first of them nothing is known, so the field is not drawn as it was and a replay cannot start.

import type { TimelineEvent, TimelineKind } from "@armada/protocol";
import { expect, test } from "vitest";

import type { DroneIn, PrIn } from "./layout";
import { MOST_EVENTS, WINDOW_MS, fieldAt, neighbours, startAt, windowOf } from "./replay";

const NOW = Date.parse("2026-10-14T12:00:00Z");
const MIN = 60_000;

let count = 0;
const event = (kind: TimelineKind, subject: TimelineEvent["subject"], ago: number, extra: Partial<TimelineEvent> = {}): TimelineEvent => {
  count += 1;
  return { id: count, at: new Date(NOW - ago).toISOString(), kind, subject, tone: "run", says: kind, ...extra };
};
const s1 = { kind: "session", id: "s1" } as const;
const j1 = { kind: "job", id: "j1" } as const;

const drone = (key: string): DroneIn => ({ key, kind: key.startsWith("job:") ? "job" : "session", title: key, root: "/r", said: "now", hue: "calm", spin: false, hover: false, ping: false, stuck: false, flare: false, working: false, touches: [], shown: true });
const none = () => undefined;

test("the window is the last two hours of events, oldest first, and a replay starts at the first of them", () => {
  const window = windowOf([event("session.turn", s1, 30 * MIN), event("session.started", s1, 90 * MIN), event("job.created", j1, WINDOW_MS + 5 * MIN)], NOW);

  expect(window?.events.map((one) => one.kind)).toEqual(["session.started", "session.turn"]);
  expect(window?.from).toBe(NOW - 90 * MIN);
});

test("no event in the window is no replay", () => {
  expect(windowOf([], NOW)).toBeUndefined();
  expect(windowOf([event("job.created", j1, WINDOW_MS + MIN)], NOW)).toBeUndefined();
});

test("at most 2000 events, the newest, so the start moves up with them", () => {
  const many = Array.from({ length: MOST_EVENTS + 500 }, (_, at) => event("session.turn", s1, (MOST_EVENTS + 500 - at) * 1000));
  const window = windowOf(many, NOW);

  expect(window?.events).toHaveLength(MOST_EVENTS);
  expect(window?.from).toBe(NOW - MOST_EVENTS * 1000);
});

test("a start is never earlier than the first event, and one with no wish starts there", () => {
  const window = windowOf([event("session.started", s1, 60 * MIN)], NOW)!;

  expect(startAt(window, undefined)).toBe(NOW - 60 * MIN);
  expect(startAt(window, NOW - 100 * MIN)).toBe(NOW - 60 * MIN);
  expect(startAt(window, NOW - 10 * MIN)).toBe(NOW - 10 * MIN);
});

test("a drone whose start is in the window is absent before it; one the window finds already running is there", () => {
  const events = [event("session.started", s1, 60 * MIN), event("job.step", j1, 40 * MIN)];
  const drones = [drone("session:s1"), drone("job:j1")];

  expect(fieldAt(events, NOW - 90 * MIN, drones, [], none).drones.map((one) => one.key)).toEqual(["job:j1"]);
  expect(fieldAt(events, NOW - 50 * MIN, drones, [], none).drones.map((one) => one.key)).toEqual(["session:s1", "job:j1"]);
});

test("a drone stands as its last event up to the playhead left it, at that event's real time", () => {
  const events = [event("session.started", s1, 60 * MIN, { says: "Started" }), event("session.asked", s1, 30 * MIN, { tone: "ask", says: "Asked: which?" }), event("session.answered", s1, 20 * MIN, { tone: "run", live: true, says: "Answered" })];
  const at = (ago: number) => fieldAt(events, NOW - ago, [drone("session:s1")], [], none).drones[0]!;

  expect(at(25 * MIN).said).toBe("Asked: which?");
  expect(at(25 * MIN).hue).toBe("ask");
  expect(at(10 * MIN).said).toBe("Answered");
  expect(at(10 * MIN).hue).toBe("calm");
});

test("a pull request appears at its first event and moves along its route by the rest", () => {
  const pr = (number: number): PrIn => ({ key: `pr:${number}`, number, stage: "ready", root: "/r" });
  const on = { kind: "pull_request", number: 7 } as const;
  const events = [event("pr.opened", on, 50 * MIN), event("pr.checks", on, 40 * MIN, { tone: "bad" }), event("pr.merged", on, 10 * MIN)];
  const stage = (ago: number) => fieldAt(events, NOW - ago, [], [pr(7)], none).prs.map((one) => one.stage);

  expect(stage(60 * MIN)).toEqual([]);
  expect(stage(45 * MIN)).toEqual(["checking"]);
  expect(stage(30 * MIN)).toEqual(["red"]);
  expect(stage(5 * MIN)).toEqual(["landed"]);
});

test("the instants either side of the playhead are the events' own", () => {
  const events = [event("session.started", s1, 60 * MIN), event("session.turn", s1, 30 * MIN)];
  expect(neighbours(events, NOW - 45 * MIN)).toEqual({ before: NOW - 60 * MIN, after: NOW - 30 * MIN });
  expect(neighbours(events, NOW - 90 * MIN).before).toBeUndefined();
});
