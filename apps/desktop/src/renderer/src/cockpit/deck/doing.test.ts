// What a working drone is doing, read from the tool bursts Fleet recorded: only the latest burst, only while it can still
// be the present, and only on a drone that is working. A stale burst reads as no action.

import type { TimelineEvent, TimelineKind } from "@armada/protocol";
import { expect, test } from "vitest";

import { FRESH_MS, withDoing } from "./doing";
import type { DroneIn } from "./layout";

const NOW = Date.parse("2026-10-14T12:00:00Z");

let count = 0;
const burst = (kind: TimelineKind, subject: TimelineEvent["subject"], ago: number, extra: Partial<TimelineEvent> = {}): TimelineEvent => {
  count += 1;
  return { id: count, at: new Date(NOW - ago).toISOString(), kind, subject, tone: "run", says: kind, live: true, ...extra };
};
const drone = (key: string, working = true): DroneIn => ({
  key,
  kind: key.startsWith("job:") ? "job" : "session",
  title: key,
  root: "/r",
  said: "",
  hue: "calm",
  spin: working,
  hover: working,
  ping: false,
  stuck: false,
  flare: false,
  working,
  touches: [],
  shown: true,
});
const s1 = { kind: "session", id: "s1" } as const;
const none = () => undefined;

test("a burst that closed a moment ago is what a working drone is doing", () => {
  const [one] = withDoing([drone("session:s1")], [burst("session.edited", s1, 20_000, { detail: "crates/fleet/a.rs" })], NOW, none);
  expect(one?.doing).toBe("write");
  expect(one?.via).toBe("crates/fleet/a.rs");
});

test("a burst older than 2 minutes 15 seconds reads as no action", () => {
  const events = [burst("session.ran", s1, FRESH_MS + 1_000)];
  expect(withDoing([drone("session:s1")], events, NOW, none)[0]?.doing).toBeUndefined();
  expect(withDoing([drone("session:s1")], [burst("session.ran", s1, FRESH_MS - 1_000)], NOW, none)[0]?.doing).toBe("run");
});

test("only the latest burst counts, so a newer read replaces an older write", () => {
  const events = [burst("session.edited", s1, 90_000), burst("session.read", s1, 30_000)];
  expect(withDoing([drone("session:s1")], events, NOW, none)[0]?.doing).toBe("read");
});

test("a drone that is not working does nothing, however fresh the burst", () => {
  expect(withDoing([drone("session:s1", false)], [burst("session.read", s1, 5_000)], NOW, none)[0]?.doing).toBeUndefined();
});

test("a Job's read and run bursts count, and the kinds that are not tool bursts do not", () => {
  const j1 = { kind: "job", id: "j1" } as const;
  expect(withDoing([drone("job:j1")], [burst("job.read", j1, 10_000)], NOW, none)[0]?.doing).toBe("read");
  expect(withDoing([drone("job:j1")], [burst("job.ran", j1, 10_000)], NOW, none)[0]?.doing).toBe("run");
  expect(withDoing([drone("job:j1")], [burst("job.files", j1, 10_000), burst("job.step", j1, 5_000)], NOW, none)[0]?.doing).toBeUndefined();
});

test("a burst after the clock is not yet, and a picked burst shows whether or not it is fresh or working", () => {
  expect(withDoing([drone("session:s1")], [burst("session.read", s1, -60_000)], NOW, none)[0]?.doing).toBeUndefined();
  const old = burst("session.edited", s1, 60 * 60_000);
  const picked = withDoing([drone("session:s1", false)], [], NOW, none, { key: "session:s1", event: old })[0];
  expect(picked?.doing).toBe("write");
});

test("a failing Check turns the ring red, and a passed one green", () => {
  expect(withDoing([drone("session:s1")], [burst("session.ran", s1, 5_000, { tone: "bad" })], NOW, none)[0]?.check).toBe("bad");
  expect(withDoing([drone("session:s1")], [burst("session.ran", s1, 5_000, { tone: "ok" })], NOW, none)[0]?.check).toBe("ok");
  expect(withDoing([drone("session:s1")], [burst("session.ran", s1, 5_000)], NOW, none)[0]?.check).toBe("run");
});
