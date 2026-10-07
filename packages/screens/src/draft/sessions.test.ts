import { describe, expect, test } from "vitest";

import { isBlank, ownerOf, sessionsMatching } from "./sessions";
import type { Session } from "./sessions";

const idle = { state: "idle" } as const;

const blank: Session = { id: "s1", attachments: [], rows: [], turn: idle };
const flaky: Session = {
  id: "s2",
  title: "Flaky store test",
  turn: idle,
  rows: [],
  attachments: [
    { kind: "slot", slot: 3 },
    { kind: "branch", name: "fix/flaky-store", slot: 3 },
    { kind: "pull_request", number: 1843, title: "Pin the store clock", branch: "fix/flaky-store", address: "https://example.com/pull/1843", checks: { state: "passed" } },
    { kind: "job", id: "52", number: 52, title: "Pin the clock", state: "running", branch: "fix/52-pin-clock", slot: 4 },
  ],
};
const notes: Session = {
  id: "s3",
  title: "Release notes",
  turn: idle,
  rows: [],
  attachments: [
    { kind: "branch", name: "rel/notes", slot: 5 },
    { kind: "pull_request", number: 1847, title: "Notes script", branch: "rel/notes", address: "https://example.com/pull/1847", checks: { state: "passed" } },
  ],
};
const all = [blank, flaky, notes];
const ids = (query: string) => sessionsMatching(all, query).map((one) => one.session.id);

describe("a blank Session", () => {
  test("holds no slot and no branch", () => {
    expect(isBlank(blank)).toBe(true);
    expect(isBlank(flaky)).toBe(false);
  });
});

describe("searching Sessions", () => {
  test("#1843 finds the Session that owns that pull request", () => {
    expect(ids("#1843")).toEqual(["s2"]);
  });
  test("a hash is a pull request and not a Job id", () => {
    expect(ids("#52")).toEqual([]);
  });
  test("by branch, Job id, slot and title", () => {
    expect(ids("flaky-store")).toEqual(["s2"]);
    expect(ids("job 52")).toEqual(["s2"]);
    expect(ids("slot 4")).toEqual(["s2"]);
    expect(ids("slot 5")).toEqual([]);
    expect(ids("3")).toEqual(["s2"]);
    expect(ids("release")).toEqual(["s3"]);
  });
  test("says what matched", () => {
    expect(sessionsMatching(all, "#1843")[0]!.matched).toMatchObject({ kind: "pull_request", number: 1843 });
  });
});

describe("who owns a chip", () => {
  test("a Job's own branch and slot belong to the Session that dispatched it", () => {
    expect(ownerOf(all, { kind: "branch", name: "fix/52-pin-clock" })?.id).toBe("s2");
    expect(ownerOf(all, { kind: "slot", slot: 4 })?.id).toBe("s2");
  });
  test("a chip nobody holds has no owner", () => {
    expect(ownerOf(all, { kind: "job", id: "44" })).toBeUndefined();
  });
});
