import { describe, expect, it } from "vitest";

import { apply, diff } from "./state-change";

const roundTrip = (was: unknown, now: unknown) => {
  const patch = diff(was, now);
  return patch === undefined ? was : apply(was, patch);
};

describe("a state change", () => {
  it("is nothing where nothing moved", () => {
    expect(diff({ a: [{ id: "x", n: 1 }], b: { c: 1 } }, { a: [{ id: "x", n: 1 }], b: { c: 1 } })).toBeUndefined();
  });

  it("rebuilds what moved and keeps the rest as the same object", () => {
    const was = { jobs: [{ id: "a", n: 1 }, { id: "b", n: 2 }], other: { deep: [1, 2, 3] } };
    const now = { jobs: [{ id: "a", n: 1 }, { id: "b", n: 3 }], other: { deep: [1, 2, 3] } };
    const got = roundTrip(was, now) as typeof now;
    expect(got).toEqual(now);
    expect(got.jobs[0]).toBe(was.jobs[0]);
    expect(got.other).toBe(was.other);
  });

  it("follows rows that are added, dropped and moved by their id", () => {
    const was = { rows: [{ id: "a", n: 1 }, { id: "b", n: 2 }, { id: "c", n: 3 }] };
    const now = { rows: [{ id: "c", n: 3 }, { id: "d", n: 4 }, { id: "a", n: 9 }] };
    const got = roundTrip(was, now) as typeof now;
    expect(got).toEqual(now);
    expect(got.rows[0]).toBe(was.rows[2]);
  });

  it("drops a key that is gone and sets one that is new", () => {
    expect(roundTrip({ a: 1, b: 2 }, { a: 1, c: undefined })).toEqual({ a: 1, c: undefined });
  });

  it("replaces a list with no ids whole, and keeps one that is equal", () => {
    const was = { tags: ["a", "b"] };
    expect(roundTrip(was, { tags: ["a", "c"] })).toEqual({ tags: ["a", "c"] });
    expect((roundTrip(was, { tags: ["a", "b"] }) as typeof was).tags).toBe(was.tags);
  });

  it("replaces a value whose kind changed", () => {
    expect(roundTrip({ a: { state: "none" } }, { a: { state: "read", rows: [{ id: "x" }] } })).toEqual({ a: { state: "read", rows: [{ id: "x" }] } });
  });

  it("throws against a state it was not made for, so the window can ask again", () => {
    const patch = diff({ rows: [{ id: "a" }] }, { rows: [{ id: "a", n: 1 }] })!;
    expect(() => apply({ rows: [] }, patch)).toThrow();
  });
});
