// How a pull request's entries are placed on the timeline: on the track of the Job or Session that owns
// it, with the pull request kept on the event, and dropped where nobody owns it. Pure.

import { describe, expect, test } from "vitest";
import type { TimelineEvent } from "@armada/protocol";

import { QUIET_KINDS, fromWire, lanesOf, type FleetEvent, type Names } from "./fleet-events";

const NOW = Date.parse("2026-10-13T12:00:00Z");
const at = (minutes: number) => new Date(NOW - minutes * 60_000).toISOString();

const pull = (id: number, minutes: number, over: Partial<TimelineEvent> = {}): TimelineEvent => ({
  id,
  at: at(minutes),
  kind: "pr.checks",
  subject: { kind: "pull_request", number: 1890, url: "https://git.example/pull/1890" },
  title: "Gate policy",
  tone: "ok",
  says: "Checks passed",
  ...over,
});
const own = (id: number, minutes: number, over: Partial<TimelineEvent> = {}): TimelineEvent => ({
  id,
  at: at(minutes),
  kind: "job.step",
  subject: { kind: "job", id: "j1" },
  title: "Debounce",
  tone: "run",
  says: "Implement started",
  live: true,
  ...over,
});

type Owner = ReturnType<Names["pull"]>;
const names = (owner: Owner): Names => ({ job: () => undefined, session: () => undefined, pull: () => owner });
const drawn = (events: TimelineEvent[], owner: Owner): FleetEvent[] => events.flatMap((one) => fromWire(one, names(owner)) ?? []);

describe("a pull request's entry", () => {
  test("takes its Job's subject and keeps the pull request, its number leading the detail", () => {
    const [event] = drawn([pull(1, 5, { detail: "fleet/gate" })], { kind: "job", id: "j1", title: "Debounce" });
    expect(event?.subject).toEqual({ kind: "job", id: "j1", title: "Debounce" });
    expect(event?.pull).toEqual({ number: 1890, title: "Gate policy", url: "https://git.example/pull/1890" });
    expect(event?.detail).toBe("#1890 fleet/gate");
  });

  test("takes a Session's subject, with the number alone for a detail where the entry had none", () => {
    const [event] = drawn([pull(1, 5)], { kind: "session", id: "s1", title: "Flaky store" });
    expect(event?.subject).toEqual({ kind: "session", id: "s1", title: "Flaky store" });
    expect(event?.detail).toBe("#1890");
  });

  test("is dropped where nobody owns the pull request", () => {
    expect(fromWire(pull(1, 5), names(undefined))).toBeUndefined();
  });

  test("an entry about a Job or main carries no pull request", () => {
    expect(fromWire(own(2, 5), names(undefined))?.pull).toBeUndefined();
    expect(fromWire({ ...own(3, 5), subject: { kind: "main" } }, names(undefined))?.subject).toEqual({ kind: "main" });
  });
});

describe("the lanes a pull request's entries make", () => {
  const owner = { kind: "job", id: "j1", title: "Debounce" } as const;

  test("one lane for the owner, holding its own entries and the pull request's, and none for the pull request", () => {
    const lanes = lanesOf(drawn([own(1, 30), pull(2, 10), pull(3, 5, { kind: "pr.queued", says: "Queued, place 1" })], owner), NOW);
    expect(lanes.map((one) => one.key)).toEqual(["job:j1"]);
    expect(lanes[0]?.events.map((one) => one.says)).toEqual(["Queued, place 1", "Checks passed", "Implement started"]);
  });

  test("how the lane stands is read from its own entries: a pull request after the Session ended does not reopen it", () => {
    const session = { kind: "session", id: "s1", title: "Flaky store" } as const;
    const ended = own(1, 30, { kind: "session.ended", subject: { kind: "session", id: "s1" }, says: "Ended", tone: "idle", live: false, over: true });
    // Both entries are read through one owner, so the ended entry is the Session's own.
    const events = [...drawn([ended], undefined), ...drawn([pull(2, 5, { kind: "pr.opened", says: "Opened", tone: "idle" })], session)];
    const lanes = lanesOf(events, NOW);
    expect(lanes[0]?.standing).toBe("done");
    expect(lanes[0]?.head.says).toBe("Ended");
  });

  test("a pull request nobody owns makes no lane", () => {
    expect(lanesOf(drawn([own(1, 30), pull(2, 5)], undefined), NOW).map((one) => one.key)).toEqual(["job:j1"]);
  });
});

describe("what a Session or a Job's Drone did between its turns", () => {
  const session = { kind: "session", id: "s1" } as const;
  const burst = (kind: TimelineEvent["kind"], says: string, over: Partial<TimelineEvent> = {}) => own(1, 5, { kind, subject: session, says, ...over });

  test("every kind of the burst and of the agent's own list is read, with the files or commands a burst counted", () => {
    const kinds: [TimelineEvent["kind"], string][] = [
      ["session.read", "Read 2 files"],
      ["session.edited", "Edited 2 files"],
      ["session.ran", "Ran 2 commands"],
      ["job.read", "Read 2 files"],
      ["job.ran", "Ran 2 commands"],
    ];
    for (const [kind, says] of kinds) {
      const event = fromWire(burst(kind, says, { detail: "b", items: ["b", "a"] }), names(undefined));
      expect(event?.kind).toBe(kind);
      expect(event?.items).toEqual(["b", "a"]);
      expect(event?.detail).toBe("b");
      expect(QUIET_KINDS.has(kind)).toBe(true);
    }
    for (const kind of ["session.subagent", "session.waiting", "session.resolved", "session.spawned"] as const) {
      expect(fromWire(burst(kind, "x"), names(undefined))?.kind).toBe(kind);
      expect(QUIET_KINDS.has(kind)).toBe(false);
    }
  });

  test("an entry written before Fleet kept the items, or kept none, has nothing to open", () => {
    expect(fromWire(burst("session.read", "Read 14 files", { detail: "b" }), names(undefined))).not.toHaveProperty("items");
    expect(fromWire(burst("session.read", "Read 14 files", { detail: "b", items: [] }), names(undefined))).not.toHaveProperty("items");
  });

  test("a Session waiting on him needs him, as one asked does, by its tone alone", () => {
    const waiting = burst("session.waiting", "Waiting: approve #1896", { tone: "ask", live: false });
    const asked = burst("session.asked", "Asked: manifest hash or path?", { tone: "ask", live: false });
    for (const head of [waiting, asked]) {
      const [lane] = lanesOf([fromWire(burst("session.read", "Read 3 files"), names(undefined))!, fromWire({ ...head, at: at(1) }, names(undefined))!], NOW);
      expect(lane?.standing).toBe("ask");
    }
  });
});

describe("a Session's lane stands by what Fleet says it is doing", () => {
  const session = { kind: "session", id: "s1", title: "Flaky store" } as const;
  const turn = (minutes: number) => own(1, minutes, { kind: "session.turn", subject: { kind: "session", id: "s1" }, tone: "idle", says: "Turn took 2m 14s", live: true });
  const lane = (minutes: number, reading?: "working" | "stuck" | "replied" | "idle") =>
    lanesOf(drawn([turn(minutes)], session), NOW, reading === undefined ? undefined : () => reading)[0];

  test("an old live-flagged turn is not stuck and not moving unless Fleet says so", () => {
    expect(lane(90)?.standing).toBe("idle");
    expect(lane(90, "idle")?.standing).toBe("idle");
  });

  test("Fleet's stuck is stuck, however fresh the entry, and a reply asks nothing", () => {
    expect(lane(1, "stuck")?.standing).toBe("stuck");
    expect(lane(90, "replied")?.standing).toBe("idle");
    expect(lane(90, "working")?.standing).toBe("run");
  });
});
