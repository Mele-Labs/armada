// Events as motion: only recent ones move, only the kinds Fleet has play, and a pull request's event plays on its owner.

import type { TimelineEvent, TimelineKind } from "@armada/protocol";
import { expect, test } from "vitest";

import { MOST_PLAYING, PLAYS, RECENT_MS, TRAIL_MS, playsOf, trailsOf } from "./plays";

const NOW = Date.parse("2026-10-14T12:00:00Z");
let count = 0;
const event = (kind: TimelineKind, subject: TimelineEvent["subject"], ago: number): TimelineEvent => {
  count += 1;
  return { id: count, at: new Date(NOW - ago).toISOString(), kind, subject, tone: "idle", says: kind };
};
const s1 = { kind: "session", id: "s1" } as const;
const none = () => undefined;

test("a lease launches a drone from its bay, and the tool bursts are not plays", () => {
  expect(PLAYS["slot.leased"]).toBe("launch");
  for (const kind of ["session.read", "session.edited", "session.ran"] as const) expect(PLAYS[kind]).toBeUndefined();
});

test("only events within six minutes play, newest first, spaced apart and no more than a dozen", () => {
  const events = [event("slot.leased", s1, RECENT_MS + 1000), event("session.started", s1, 60_000), event("session.asked", s1, 10_000), ...Array.from({ length: 20 }, () => event("session.turn", s1, 30_000))];
  const plays = playsOf(events, NOW, none, none);

  expect(plays).toHaveLength(MOST_PLAYING);
  expect(plays[0]!.kind).toBe("blip");
  expect(plays.map((one) => one.delay)).toEqual(plays.map((_, at) => at * 220));
  expect(playsOf([event("slot.leased", s1, RECENT_MS + 1000)], NOW, none, none)).toEqual([]);
});

test("a pull request's event plays on the drone that owns it, or on none", () => {
  const on = { kind: "pull_request", number: 7 } as const;
  const merged = event("pr.merged", on, 5_000);

  expect(playsOf([merged], NOW, (number) => (number === 7 ? "session:s1" : undefined), none)[0]).toMatchObject({ kind: "capture", key: "session:s1", number: 7 });
  expect(playsOf([merged], NOW, none, none)[0]?.key).toBeUndefined();
});

test("a drone's contrail is the age of its last dispatch, and none after 45 minutes", () => {
  const trails = trailsOf([event("session.started", s1, 10 * 60_000), event("slot.leased", s1, 4 * 60_000), event("session.started", { kind: "session", id: "old" }, TRAIL_MS + 1000)], NOW, none);

  expect(trails.get("session:s1")).toBe(4 * 60_000);
  expect(trails.has("session:old")).toBe(false);
});
