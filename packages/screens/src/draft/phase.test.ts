// A step's phase is its own where it declares one, and what `delivers` says where it does not.

import { describe, expect, it } from "vitest";

import { phaseOf, phased } from "./phase";

describe("phaseOf", () => {
  it("reads a declared phase over what the step delivers", () => {
    expect(phaseOf(phased({ delivers: false }, "delivery"))).toBe("delivery");
    expect(phaseOf(phased({ delivers: true }, "work"))).toBe("work");
  });

  it("falls back to delivers, never to the id", () => {
    expect(phaseOf({ delivers: true })).toBe("delivery");
    expect(phaseOf({ delivers: false })).toBe("work");
  });
});
