import { describe, expect, test } from "vitest";

import { mainOwner, pullOwner } from "./owner";

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
