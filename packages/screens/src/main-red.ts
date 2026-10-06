// A Job's part in main's red: it took the red, or it fixed it. Drawn ahead of Fleet, so it rides
// on the row as a field the wire does not carry yet, the way `notice` rides on a merge line. A row
// from a Fleet that serves none has none, and nothing here draws.

import type { JobSummary } from "@armada/protocol";

/** The Job working on main's red, or the one that fixed it. */
export type FixesMain = {
  state: "fixing" | "fixed";
  /** The Check that is red on main. */
  check: string;
  /** Absent where Armada could not read a failing test out of the log. */
  test?: string;
  /** The pull request that turned main red. */
  merge: number;
  /** The pull request that fixed it, once `state` is `fixed`. */
  fixed_in?: number;
};

type Fixing = JobSummary & { fixes_main?: FixesMain };

export function fixesMainOf(job: JobSummary): FixesMain | undefined {
  return (job as Fixing).fixes_main;
}

/** What the mark beside the badge says: what main is red for, or the pull request that fixed it. */
export function fixesMainSaid(fixes: FixesMain): string {
  return fixes.state === "fixing"
    ? `Fixing main: ${fixes.check} failed after #${fixes.merge}`
    : `Fixed main in #${fixes.fixed_in}`;
}
