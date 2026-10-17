// What Setup says about `setup.seed`, on the Manifest surface. #1064. Fleet writes the reason a
// worktree started cold; this only frames it.

import type { DeclaredSeed } from "@armada/protocol";

const short = (commit: string): string => commit.slice(0, 12);

/** The Manifest surface's Setup line. Absent where the Manifest declares no seed. */
export function seedSaid(seed: DeclaredSeed | undefined): string | undefined {
  if (seed === undefined) return undefined;
  const declared = `A new worktree is seeded with ${seed.paths.join(", ")}, warmed by ${seed.warmed_by.join(", ")}.`;
  switch (seed.warmth.state) {
    case "warm":
      return `${declared} Warm at base ${short(seed.warmth.commit)}.`;
    case "warming":
      return `${declared} Warming at base ${short(seed.warmth.commit)}, so a worktree cut now starts cold.`;
    case "cold":
      return `${declared} Not warm: ${seed.warmth.why}.`;
  }
}
