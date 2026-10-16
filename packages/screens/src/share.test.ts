import { describe, expect, it } from "vitest";

import { share } from "./share";

describe("what the window already holds", () => {
  it("is kept wherever the new value equals it", () => {
    const was = { rows: [{ id: "a", text: "one" }, { id: "b", text: "two" }], title: "t" };
    const now = structuredClone(was);
    expect(share(was, now)).toBe(was);
  });

  it("is replaced only where something changed, and a row that moved is still found by its id", () => {
    const was = [{ id: "a", text: "one" }, { id: "b", text: "two" }];
    const now = share(was, [{ id: "z", text: "new" }, { id: "a", text: "one" }, { id: "b", text: "TWO" }]);
    expect(now[1]).toBe(was[0]);
    expect(now[2]).not.toBe(was[1]);
    expect(now[2]).toEqual({ id: "b", text: "TWO" });
  });

  it("takes the new value where the old one was another kind of thing", () => {
    expect(share("x", { a: 1 })).toEqual({ a: 1 });
    expect(share(undefined, [1])).toEqual([1]);
  });
});
