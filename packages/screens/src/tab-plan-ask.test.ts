// What a proposed change tells the Drone. `#1552`.

import { describe, expect, test } from "vitest";

import { arcGroups } from "./fixtures/build/arc-plan";
import { proposeInstruction, rewriteInstruction } from "./tab-plan-ask";

const GROUPS = arcGroups();

describe("what the Drone is told", () => {
  test("a rewrite is addressed to its task and says a refusal is an answer", () => {
    const said = rewriteInstruction("T5", "split the rows out");
    expect(said).toContain("on T5: split the rows out");
    expect(said).toContain("refuse it and say what that reason is");
  });

  test("a proposal on a group names the group and the tasks it holds", () => {
    expect(proposeInstruction(GROUPS, "g4", "keep them apart")).toContain("on group 4 (T7, T8): keep them apart");
  });
});
