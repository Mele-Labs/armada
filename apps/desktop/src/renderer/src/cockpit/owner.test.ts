import { describe, expect, test } from "vitest";

import { failedAt, isOnIt, mainOwner, pullOwner, timelineOwner, workOwner, type Owner } from "./owner";

type Views = Parameters<typeof pullOwner>[0];
type Sessions = Parameters<typeof pullOwner>[1];

const views = (pulls: unknown[], main?: unknown): Views => [{ root: "/armada", hub: { pulls, ...(main === undefined ? {} : { main }) } }] as unknown as Views;
const session = (id: string, title: string, attachments: unknown[]): Sessions[number] => ({ id, title, attachments, rows: [] }) as unknown as Sessions[number];
const holds = (number: number, branch = "x/y") => ({ kind: "pull_request", number, branch });

describe("who owns a pull request", () => {
  test("a Job that opened it owns it", () => {
    const seen = views([{ number: 1819, branch: "fleet/pause", job: { id: "j1", title: "Store a pause marker" } }]);
    expect(pullOwner(seen, [], 1819)).toEqual({ kind: "job", id: "j1", title: "Store a pause marker" });
  });

  test("a Session that holds it owns it, found by its number", () => {
    const seen = views([{ number: 1822, branch: "pocket/1998-pwa" }]);
    expect(pullOwner(seen, [session("s13", "Armada Pocket", [holds(1822)])], 1822)).toEqual({ kind: "session", id: "s13", title: "Armada Pocket" });
  });

  test("a Session that holds its branch owns it, where no number is attached yet", () => {
    const seen = views([{ number: 1822, branch: "pocket/1998-pwa" }]);
    expect(pullOwner(seen, [session("s13", "Armada Pocket", [holds(0, "pocket/1998-pwa")])], 1822, "pocket/1998-pwa")?.id).toBe("s13");
  });

  test("a Job wins over a Session, since the line's own row says so", () => {
    const seen = views([{ number: 5, branch: "b", job: { id: "j", title: "T" } }]);
    expect(pullOwner(seen, [session("s", "S", [holds(5)])], 5)?.kind).toBe("job");
  });

  test("a person's pull request has no owner, and another Session's does not count", () => {
    const seen = views([{ number: 1823, branch: "chore/bump-lockfile" }]);
    expect(pullOwner(seen, [session("s13", "Armada Pocket", [holds(1822)])], 1823)).toBeUndefined();
  });
});

describe("who owns main's red", () => {
  const red = (merge?: unknown) => ({ state: "red", red: { check: "desktop_test", ...(merge === undefined ? {} : { merge }) } });

  test("the Job whose branch broke it", () => {
    const seen = views([], red({ number: 1814, branch: "studio/zone", url: "u", job: { id: "j2", title: "Name the zone" } }));
    expect(mainOwner(seen, [], "/armada")).toEqual({ kind: "job", id: "j2", title: "Name the zone" });
  });

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
  const seen = views([{ number: 7, branch: "feat/seven" }]);
  const jobs = [{ id: "j9", title: "Seven by branch", branch: "feat/seven" }];
  const branched = (name: string) => ({ kind: "branch", name });

  test("the Job the wire names comes first, before a Job on its branch or a Session that holds it", () => {
    const named = views([{ number: 7, branch: "feat/seven", job: { id: "j1", title: "Named" } }]);
    expect(workOwner(named, jobs, [session("s1", "S", [holds(7)])], 7, "feat/seven")).toEqual({ kind: "job", id: "j1", title: "Named" });
  });

  test("a Job on its branch comes before a Session that holds it", () => {
    expect(workOwner(seen, jobs, [session("s1", "S", [holds(7)])], 7, "feat/seven")).toEqual({ kind: "job", id: "j9", title: "Seven by branch" });
  });

  test("a Session holds it by its number, its pull request's branch, or the branch it works on", () => {
    expect(workOwner(seen, [], [session("s1", "S", [holds(7)])], 7, "feat/seven")?.id).toBe("s1");
    expect(workOwner(seen, [], [session("s2", "S", [holds(0, "feat/seven")])], 7, "feat/seven")?.id).toBe("s2");
    expect(workOwner(seen, [], [session("s3", "S", [branched("feat/seven")])], 7, "feat/seven")?.id).toBe("s3");
  });

  test("a live Session comes before one that ended, and an ended one still owns what no live one holds", () => {
    const ended = { ...session("old", "Old", [holds(7)]), dead: "ended" } as unknown as Sessions[number];
    const live = session("live", "Live", [branched("feat/seven")]);
    expect(workOwner(seen, [], [ended, live], 7, "feat/seven")?.id).toBe("live");
    expect(workOwner(seen, [], [ended], 7, "feat/seven")?.id).toBe("old");
  });

  test("a Session without a title is called by its id, and nobody owns a person's pull request", () => {
    const blank = { id: "s5", attachments: [holds(7)], rows: [] } as unknown as Sessions[number];
    expect(workOwner(seen, [], [blank], 7, "feat/seven")).toEqual({ kind: "session", id: "s5", title: "s5" });
    expect(workOwner(seen, jobs, [session("s1", "S", [holds(8, "other")])], 7, "chore/by-hand")).toBeUndefined();
  });
});

describe("whether the owner is already on the failure", () => {
  const FAILED = Date.parse("2026-10-13T13:00:00Z");
  const before = "2026-10-13T12:50:00Z";
  const after = "2026-10-13T13:05:00Z";
  const owns: Owner = { kind: "session", id: "s1", title: "Pin the clock" };
  const message = (from: "you" | "agent" | "session", instant: string) => ({ kind: "message", from: { kind: from }, at: "00:00:00", instant });
  const held = (turn: "working" | "idle", rows: unknown[] = [], extra: object = {}) =>
    [{ id: "s1", title: "Pin the clock", turn: { state: turn }, rows, attachments: [], ...extra }] as unknown as Sessions;
  const nobody = new Set<string>();
  const untold = new Map<string, string>();

  test("a Session at work is on it, whatever was said", () => {
    expect(isOnIt(owns, held("working"), nobody, untold, FAILED)).toBe(true);
    expect(isOnIt(owns, held("working"), nobody, untold, undefined)).toBe(true);
  });

  test("an idle Session sent anything after the failure is on it, from the owner or from another Session", () => {
    expect(isOnIt(owns, held("idle", [message("you", after)]), nobody, untold, FAILED)).toBe(true);
    expect(isOnIt(owns, held("idle", [message("session", after)]), nobody, untold, FAILED)).toBe(true);
  });

  test("what the agent said itself, or was sent before the failure, does not count", () => {
    expect(isOnIt(owns, held("idle", [message("agent", after)]), nobody, untold, FAILED)).toBe(false);
    expect(isOnIt(owns, held("idle", [message("you", before)]), nobody, untold, FAILED)).toBe(false);
  });

  test("an idle Session with nothing sent is not on it, and with no failure time nothing but work counts", () => {
    expect(isOnIt(owns, held("idle"), nobody, untold, FAILED)).toBe(false);
    expect(isOnIt(owns, held("idle", [message("you", after)]), nobody, untold, undefined)).toBe(false);
  });

  test("a Session that ended, or that is not there, is not on it", () => {
    expect(isOnIt(owns, held("working", [], { dead: "ended" }), nobody, untold, FAILED)).toBe(false);
    expect(isOnIt(owns, [], nobody, untold, FAILED)).toBe(false);
  });

  const job: Owner = { kind: "job", id: "j1", title: "Name the zone" };

  test("a Job is on it while it runs", () => {
    expect(isOnIt(job, [], new Set(["j1"]), untold, FAILED)).toBe(true);
    expect(isOnIt(job, [], new Set(["j1"]), untold, undefined)).toBe(true);
  });

  test("a Job told after the failure is on it, one told before is not, and none told is not", () => {
    expect(isOnIt(job, [], nobody, new Map([["j1", after]]), FAILED)).toBe(true);
    expect(isOnIt(job, [], nobody, new Map([["j1", before]]), FAILED)).toBe(false);
    expect(isOnIt(job, [], nobody, untold, FAILED)).toBe(false);
    expect(isOnIt(job, [], nobody, new Map([["j2", after]]), FAILED)).toBe(false);
  });

  test("a Job's telling counts only against a failure time", () => {
    expect(isOnIt(job, [], nobody, new Map([["j1", after]]), undefined)).toBe(false);
  });

  test("main red is dated by `since`, so a Session sent anything after it first read red is on it", () => {
    const red = views([], { state: "red", red: { check: "desktop_test", since: "2026-10-13T13:00:00Z" } });
    const since = failedAt(red, "main:/armada");
    expect(since).toBe(FAILED);
    expect(isOnIt(owns, held("idle", [message("you", after)]), nobody, untold, since)).toBe(true);
    expect(isOnIt(owns, held("idle", [message("you", before)]), nobody, untold, since)).toBe(false);
  });

  test("a pull request is dated by its updated_at, and main without `since` has no date", () => {
    expect(failedAt(views([{ number: 7, updatedAt: "2026-10-13T13:00:00Z" }]), "pull:7")).toBe(FAILED);
    expect(failedAt(views([], { state: "red", red: { check: "x" } }), "main:/armada")).toBeUndefined();
  });
});

describe("whose a pull request on the timeline is", () => {
  const lines = (pulls: unknown[], landed: unknown[] = [], line: unknown[] = []) =>
    [{ root: "/armada", hub: { pulls }, line, landed, sentBack: [] }] as unknown as Parameters<typeof timelineOwner>[0];
  const jobs = [{ id: "j9", title: "Pin the clock", branch: "fix/pin-clock" }];

  test("the Job the open pull request names", () => {
    const seen = lines([{ number: 3, branch: "b", job: { id: "j1", title: "T" } }]);
    expect(timelineOwner(seen, [], [], 3, undefined)).toEqual({ kind: "job", id: "j1", title: "T" });
  });

  test("the Job a landed row names, once the pull request has left the open list", () => {
    const seen = lines([], [{ branch: "b", state: "landed", pr: { number: 4, url: "u" }, job: { id: "j2", title: "Landed" } }]);
    expect(timelineOwner(seen, [], [], 4, undefined)).toEqual({ kind: "job", id: "j2", title: "Landed" });
  });

  test("the Job whose branch the entry carried, and a Session that works on the branch", () => {
    expect(timelineOwner(lines([]), jobs, [], 5, "fix/pin-clock")).toEqual({ kind: "job", id: "j9", title: "Pin the clock" });
    const held = [session("s1", "Flaky store", [{ kind: "branch", name: "fix/flaky" }])];
    expect(timelineOwner(lines([]), jobs, held, 6, "fix/flaky")).toEqual({ kind: "session", id: "s1", title: "Flaky store" });
  });

  test("the branch the open list gives the number, where the entry carried none", () => {
    const seen = lines([{ number: 7, branch: "fix/pin-clock" }]);
    expect(timelineOwner(seen, jobs, [], 7, undefined)?.id).toBe("j9");
  });

  test("nobody, where no Job or Session is on it", () => {
    expect(timelineOwner(lines([{ number: 8, branch: "chore/bump" }]), jobs, [session("s2", "Other", [])], 8, undefined)).toBeUndefined();
    expect(timelineOwner(lines([]), jobs, [], 9, undefined)).toBeUndefined();
    expect(timelineOwner(lines([]), jobs, [], 10, "chore/else")).toBeUndefined();
  });
});
