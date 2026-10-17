// What Setup says about `setup.seed`, on the Manifest surface. #1064.

import { describe, expect, it } from "vitest";

import type { DeclaredSeed } from "@armada/protocol";
import { seedSaid } from "./seed";

const COMMIT = "a787ffc2c1d0aa00000000000000000000000000";

const DECLARED: DeclaredSeed = {
  paths: ["target"],
  warmed_by: ["warm_build", "warm_tests"],
  warmth: { state: "warm", commit: COMMIT },
};

describe("seedSaid", () => {
  it("says nothing where the Manifest declares no seed", () => {
    expect(seedSaid(undefined)).toBeUndefined();
  });

  it("names the directories, the warm-up and how warm it is", () => {
    expect(seedSaid(DECLARED)).toBe(
      "A new worktree is seeded with target, warmed by warm_build, warm_tests. Warm at base a787ffc2c1d0.",
    );
  });

  it("says a worktree cut during a warm-up starts cold", () => {
    expect(seedSaid({ ...DECLARED, warmth: { state: "warming", commit: COMMIT } })).toContain(
      "a worktree cut now starts cold",
    );
  });
});
