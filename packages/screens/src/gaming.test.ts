// The gaming check's reading: what the rail and the panel say about it. #1079.

import { describe, expect, it } from "vitest";
import type { DeclaredJudge, Flagged } from "@armada/protocol";

import { freshStep } from "./fixtures/build/base";
import {
  declaredPatterns,
  flagsOf,
  gamingStands,
  gamingSummary,
} from "./gaming";

const FILE = "packages/settings/test/useColumnSelectors.test.ts";

function flag(over: Partial<Flagged> = {}): Flagged {
  return { attempt: 1, pattern: "assertion_weakened", cited: "", at: { file: FILE }, ...over };
}

describe("what the rail and the panel say", () => {
  const held = flag();
  const cleared: Flagged = { ...flag(), cleared: { why: "a doc comment, not a check" } };

  it("splits the flags that hold the step from those a second reading cleared", () => {
    const read = flagsOf(freshStep("implement", "Implement", 2), [held, cleared]);
    expect(read.held).toEqual([held]);
    expect(read.cleared).toEqual([cleared]);
  });

  it("says on the rail that the gaming check stopped the step", () => {
    expect(gamingStands({ held: [held], cleared: [] }, true)).toBe("1 flag · stopped here");
    expect(gamingStands({ held: [], cleared: [cleared, cleared] }, false)).toBe("2 flags cleared");
    expect(gamingStands({ held: [], cleared: [] }, false)).toBeUndefined();
  });

  it("says in the panel what it flagged, and never a count of patterns it cannot see", () => {
    expect(gamingSummary({ held: [held], cleared: [] }, true, true)).toBe("1 flagged · stopped the step");
    expect(gamingSummary({ held: [], cleared: [cleared] }, true, false)).toBe("1 cleared");
    expect(gamingSummary({ held: [], cleared: [] }, true, false)).toBe("nothing flagged");
    expect(gamingSummary({ held: [], cleared: [] }, false, false)).toBe("not reached");
  });

  it("counts flagged patterns against every declared one, where Fleet names them", () => {
    const declared = ["assertion_weakened", "test_skipped", "test_deleted", "tautological_test", "check_config_edited", "test_scope_narrowed"];
    expect(gamingSummary({ held: [held, held], cleared: [] }, true, true, declared)).toBe(
      "1 of 6 flagged · stopped the step",
    );
    expect(gamingSummary({ held: [], cleared: [] }, true, false, declared)).toBe("0 of 6 flagged");
    expect(gamingSummary({ held: [], cleared: [] }, false, false, declared)).toBe("not reached");
  });

  it("reads the declared patterns in order, and nothing from a Fleet that does not name them", () => {
    const naming: DeclaredJudge = {
      criteria: 0,
      gaming_check: true,
      gaming_patterns: ["test_deleted", "assertion_weakened"],
    };
    expect(declaredPatterns({ ...freshStep("implement", "Implement", 2), judge_checks: [naming] })).toEqual([
      "test_deleted",
      "assertion_weakened",
    ]);
    const silent = { ...freshStep("implement", "Implement", 2), judge_checks: [{ criteria: 0, gaming_check: true }] };
    expect(declaredPatterns(silent)).toBeUndefined();
  });
});
