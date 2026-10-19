import { describe, expect, test } from "vitest";

import { failedAt, isOnIt, mainOwner, pullOwner, timelineOwner, workOwner, type Owner } from "./owner";

type Views = Parameters<typeof mainOwner>[0];
type Sessions = Parameters<typeof pullOwner>[0];

const views = (pulls: unknown[], main?: unknown): Views => [{ root: "/armada", hub: { pulls, ...(main === undefined ? {} : { main }) } }] as unknown as Views;
const session = (id: string, title: string, attachments: unknown[]): Sessions[number] => ({ id, title, attachments, rows: [] }) as unknown as Sessions[number];
const holds = (number: number, branch = "x/y") => ({ kind: "pull_request", number, branch });

describe("who owns a pull request", () => {
  test("a Session that holds it owns it, found by its number", () => {
    expect(pullOwner([session("s13", "Armada Pocket", [holds(1822)])], 1822)).toEqual({ kind: "session", id: "s13", title: "Armada Pocket" });
  });

  test("a Session that holds its branch owns it, where no number is attached yet", () => {
    expect(pullOwner([session("s13", "Armada Pocket", [holds(0, "pocket/1998-pwa")])], 1822, "pocket/1998-pwa")?.id).toBe("s13");
  });

  test("a person's pull request has no owner, and another Session's does not count", () => {
    expect(pullOwner([session("s13", "Armada Pocket", [holds(1822)])], 1823)).toBeUndefined();
  });
});

describe("who owns main's red", () => {
  const red = (merge?: unknown) => ({ state: "red", red: { check: "desktop_test", ...(merge === undefined ? {} : { merge }) } });

  test("a Session that holds the pull request that merged it", () => {
    const seen = views([], red({ number: 1816, branch: "x/y", url: "u" }));
    expect(mainOwner(seen, [session("s1", "Pin the clock", [holds(1816)])], "/armada")?.kind).toBe("session");
  });

  test("nobody, where a person merged it by hand or main is not red", () => {
    expect(mainOwner(views([], red({ number: 1815, branch: "nick/theme", url: "u" })), [], "/armada")).toBeUndefined();
    expect(mainOwner(views([], red()), [], "/armada")).toBeUndefined();
    expect(mainOwner(views([], { state: "green" }), [], "/armada")).toBeUndefined();
  });
});

describe("whose work a pull request is", () => {
  const branched = (name: string) => ({ kind: "branch", name });

  test("a Session holds it by its number, its pull request's branch, or the branch it works on", () => {
    expect(workOwner([session("s1", "S", [holds(7)])], 7, "feat/seven")?.id).toBe("s1");
    expect(workOwner([session("s2", "S", [holds(0, "feat/seven")])], 7, "feat/seven")?.id).toBe("s2");
    expect(workOwner([session("s3", "S", [branched("feat/seven")])], 7, "feat/seven")?.id).toBe("s3");
  });

  test("a live Session comes before one that ended, and an ended one still owns what no live one holds", () => {
    const ended = { ...session("old", "Old", [holds(7)]), dead: "ended" } as unknown as Sessions[number];
    const live = session("live", "Live", [branched("feat/seven")]);
    expect(workOwner([ended, live], 7, "feat/seven")?.id).toBe("live");
    expect(workOwner([ended], 7, "feat/seven")?.id).toBe("old");
  });

  test("a Session without a title is called by its id, and nobody owns a person's pull request", () => {
    const blank = { id: "s5", attachments: [holds(7)], rows: [] } as unknown as Sessions[number];
    expect(workOwner([blank], 7, "feat/seven")).toEqual({ kind: "session", id: "s5", title: "s5" });
    expect(workOwner([session("s1", "S", [holds(8, "other")])], 7, "chore/by-hand")).toBeUndefined();
  });
});

describe("whether the owner is already on the failure", () => {
  const FAILED = Date.parse("2026-10-13T13:00:00Z");
  const before = "2026-10-13T12:50:00Z";
  const after = "2026-10-13T13:05:00Z";
  const owns: Owner = { kind: "session", id: "s1", title: "Pin the clock" };
  const message = (from: "you" | "agent" | "session", instant: string) => ({ kind: "message", from: { kind: from }, at: instant });
  const held = (turn: "working" | "idle", rows: unknown[] = [], extra: object = {}) =>
    [{ id: "s1", title: "Pin the clock", turn: { state: turn }, rows, attachments: [], ...extra }] as unknown as Sessions;

  test("a Session at work is on it, whatever was said", () => {
    expect(isOnIt(owns, held("working"), FAILED)).toBe(true);
    expect(isOnIt(owns, held("working"), undefined)).toBe(true);
  });

  test("an idle Session sent anything after the failure is on it, from the owner or from another Session", () => {
    expect(isOnIt(owns, held("idle", [message("you", after)]), FAILED)).toBe(true);
    expect(isOnIt(owns, held("idle", [message("session", after)]), FAILED)).toBe(true);
  });

  test("what the agent said itself, or was sent before the failure, does not count", () => {
    expect(isOnIt(owns, held("idle", [message("agent", after)]), FAILED)).toBe(false);
    expect(isOnIt(owns, held("idle", [message("you", before)]), FAILED)).toBe(false);
  });

  test("an idle Session with nothing sent is not on it, and with no failure time nothing but work counts", () => {
    expect(isOnIt(owns, held("idle"), FAILED)).toBe(false);
    expect(isOnIt(owns, held("idle", [message("you", after)]), undefined)).toBe(false);
  });

  test("a Session that ended, or that is not there, is not on it", () => {
    expect(isOnIt(owns, held("working", [], { dead: "ended" }), FAILED)).toBe(false);
    expect(isOnIt(owns, [], FAILED)).toBe(false);
  });

  test("main red is dated by `since`, so a Session sent anything after it first read red is on it", () => {
    const red = views([], { state: "red", red: { check: "desktop_test", since: "2026-10-13T13:00:00Z" } });
    const since = failedAt(red, "main:/armada");
    expect(since).toBe(FAILED);
    expect(isOnIt(owns, held("idle", [message("you", after)]), since)).toBe(true);
    expect(isOnIt(owns, held("idle", [message("you", before)]), since)).toBe(false);
  });

  test("a pull request is dated by its updated_at, and main without `since` has no date", () => {
    expect(failedAt(views([{ number: 7, updatedAt: "2026-10-13T13:00:00Z" }]), "pull:7")).toBe(FAILED);
    expect(failedAt(views([], { state: "red", red: { check: "x" } }), "main:/armada")).toBeUndefined();
  });
});

describe("whose a pull request on the timeline is", () => {
  const lines = (pulls: unknown[], landed: unknown[] = [], line: unknown[] = []) =>
    [{ root: "/armada", hub: { pulls }, line, landed, sentBack: [] }] as unknown as Parameters<typeof timelineOwner>[0];

  test("a Session that works on the branch the entry carried", () => {
    const held = [session("s1", "Flaky store", [{ kind: "branch", name: "fix/flaky" }])];
    expect(timelineOwner(lines([]), held, 6, "fix/flaky")).toEqual({ kind: "session", id: "s1", title: "Flaky store" });
  });

  test("the branch the open list gives the number, where the entry carried none", () => {
    const seen = lines([{ number: 7, branch: "fix/pin-clock" }]);
    const held = [session("s1", "Pin the clock", [{ kind: "branch", name: "fix/pin-clock" }])];
    expect(timelineOwner(seen, held, 7, undefined)?.id).toBe("s1");
  });

  test("nobody, where no Session is on it", () => {
    expect(timelineOwner(lines([{ number: 8, branch: "chore/bump" }]), [session("s2", "Other", [])], 8, undefined)).toBeUndefined();
    expect(timelineOwner(lines([]), [], 9, undefined)).toBeUndefined();
    expect(timelineOwner(lines([]), [], 10, "chore/else")).toBeUndefined();
  });
});
