// The part of a thread a fork shows of the Session it came from: up to the message it was cut at.

import { describe, expect, it } from "vitest";

import { upTo } from "./session-thread-rows";

const rows = ["a", "b", "c", "d"].map((id) => ({ id }));

describe("the old Session's rows a fork shows", () => {
  it("ends at the message the fork was cut at, that message included", () => {
    expect(upTo(rows, "b").map((one) => one.id)).toEqual(["a", "b"]);
  });

  it("is the whole thread for a fork of the whole of an ended Session", () => {
    expect(upTo(rows, undefined)).toBe(rows);
  });

  it("shows nothing where the cut cannot be found, and never what came after it", () => {
    expect(upTo(rows, "gone")).toEqual([]);
    expect(upTo([], "b")).toEqual([]);
  });
});
