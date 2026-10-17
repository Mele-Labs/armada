// What the approval canvas opens on, and one step moved without the others.

import { describe, expect, it } from "vitest";

import { checksOffWith, deliveryOf, tunedStep, tuningOf } from "./tuning";

describe("tuningOf", () => {
  it("reads a step's Judges off its panel, one where it declares none", () => {
    const tuning = tuningOf(
      [
        { step_id: "plan", judge_checks: [{ criteria: 2, gaming_check: false, panel_size: 3 }] },
        { step_id: "handoff" },
      ],
    );
    expect(tuning.steps["plan"]?.judges).toBe(3);
    expect(tuning.steps["handoff"]?.judges).toBe(1);
    expect(tuning.steps["plan"]?.harness).toBeNull();
  });
});

describe("tunedStep", () => {
  it("moves one step and carries the rest", () => {
    const tuning = tuningOf([{ step_id: "plan" }, { step_id: "implement" }]);
    const moved = tunedStep(tuning, "plan", { model: "opus" });
    expect(moved.steps["plan"]?.model).toBe("opus");
    expect(moved.steps["implement"]).toEqual(tuning.steps["implement"]);
  });
});

describe("checksOffWith", () => {
  it("turns a Check off once, and back on", () => {
    const off = checksOffWith(checksOffWith([], "build", false), "build", false);
    expect(off).toEqual(["build"]);
    expect(checksOffWith(off, "build", true)).toEqual([]);
  });
});

describe("deliveryOf", () => {
  it("is local over whatever pr_mode holds", () => {
    expect(deliveryOf(true, "draft")).toBe("local");
    expect(deliveryOf(false, "draft")).toBe("draft");
  });
});
