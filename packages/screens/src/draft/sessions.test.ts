import { describe, expect, test } from "vitest";

import { isBlank, opensPullRequests, ownerOf, sessionsMatching } from "./sessions";
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
    { kind: "pull_request", number: 1843, title: "Pin the store clock", branch: "fix/flaky-store", address: "https://example.com/pull/1843", checks: { state: "passed" }, state: "open", auto: false },
  ],
};
const notes: Session = {
  id: "s3",
  title: "Release notes",
  turn: idle,
  rows: [],
  attachments: [
    { kind: "branch", name: "rel/notes", slot: 5 },
    { kind: "pull_request", number: 1847, title: "Notes script", branch: "rel/notes", address: "https://example.com/pull/1847", checks: { state: "passed" }, state: "open", auto: false },
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
  test("a hash is a pull request and nothing else", () => {
    expect(ids("#3")).toEqual([]);
  });
  test("by branch, slot and title", () => {
    expect(ids("flaky-store")).toEqual(["s2"]);
    expect(ids("slot 3")).toEqual(["s2"]);
    expect(ids("slot 5")).toEqual([]);
    expect(ids("3")).toEqual(["s2"]);
    expect(ids("release")).toEqual(["s3"]);
  });
  test("says what matched", () => {
    expect(sessionsMatching(all, "#1843")[0]!.matched).toMatchObject({ kind: "pull_request", number: 1843 });
  });
});

describe("who owns a chip", () => {
  test("a Session's branch and slot belong to it", () => {
    expect(ownerOf(all, { kind: "branch", name: "fix/flaky-store" })?.id).toBe("s2");
    expect(ownerOf(all, { kind: "slot", slot: 3 })?.id).toBe("s2");
  });
  test("a chip nobody holds has no owner", () => {
    expect(ownerOf(all, { kind: "slot", slot: 44 })).toBeUndefined();
  });
  // An ended Session keeps the pull requests it made on its ledger, but holds nothing.
  test("an ended Session owns nothing, though its ledger still names the pull request", () => {
    const ended: Session = { ...notes, id: "s4", dead: "ended" };
    expect(ownerOf([ended], { kind: "pull_request", number: 1847 })).toBeUndefined();
    expect(ownerOf([ended, notes], { kind: "pull_request", number: 1847 })?.id).toBe("s3");
    expect(ended.attachments).toContainEqual(expect.objectContaining({ kind: "pull_request", number: 1847 }));
  });
});

describe("whether a Job opens pull requests", () => {
  const started = (delivery: "auto_land" | "open_pr" | "keep_local"): Session => ({ ...blank, started: { delivery } });

  test("Keep local with no pull request opened does not", () => {
    expect(opensPullRequests(started("keep_local"))).toBe(false);
    expect(opensPullRequests(blank)).toBe(false);
  });

  test("Open a pull request and Auto land do", () => {
    expect(opensPullRequests(started("open_pr"))).toBe(true);
    expect(opensPullRequests(started("auto_land"))).toBe(true);
  });

  test("Keep local does once the Job has a pull request", () => {
    expect(opensPullRequests({ ...flaky, started: { delivery: "keep_local" } })).toBe(true);
  });
});
