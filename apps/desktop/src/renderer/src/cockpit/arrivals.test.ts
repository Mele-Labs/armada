import { describe, expect, it } from "vitest";

import type { Item } from "../Dashboard";
import { alertOf, arrived } from "./arrivals";

const call = (key: string, title: string, at?: string): Item => ({ key, title, at }) as unknown as Item;

describe("arrived", () => {
  it("tells nobody on the first reading, so a call standing when Bridge opened is not news", () => {
    expect(arrived(null, [call("a", "A")])).toEqual([]);
  });

  it("is the calls the last reading did not have", () => {
    const fresh = arrived(new Set(["a"]), [call("a", "A"), call("b", "B")]);
    expect(fresh.map((one) => one.key)).toEqual(["b"]);
  });

  it("is news again when a call leaves and comes back", () => {
    expect(arrived(new Set(), [call("a", "A")]).map((one) => one.key)).toEqual(["a"]);
  });
});

describe("alertOf", () => {
  it("is nothing for nothing", () => {
    expect(alertOf([])).toBeNull();
  });

  it("names one call", () => {
    expect(alertOf([call("session:s1:q", "Fix the login")])).toEqual({
      key: "session:s1:q",
      keys: ["session:s1:q"],
      sentence: "“Fix the login” needs you.",
    });
  });

  it("counts several as one toast and lands on the one that has waited longest", () => {
    const told = alertOf([call("late", "Late", "2026-10-10T10:00:00Z"), call("early", "Early", "2026-10-10T09:00:00Z"), call("blank", "Blank")]);
    expect(told).toEqual({ key: "early", keys: ["early", "late", "blank"], sentence: "3 calls need you." });
  });
});
