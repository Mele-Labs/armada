// A Job's part in main's red: it took the red, or it fixed it. Fleet serves it on the row as
// `fixes_main` since 23.42. A row from a Fleet that serves none has none, and nothing here draws.

import type { FixesMain, JobSummary } from "@armada/protocol";

export type { FixesMain };

export function fixesMainOf(job: JobSummary): FixesMain | undefined {
  return job.fixes_main;
}

/** What the mark beside the badge says: what main is red for, or the pull request that fixed it. */
export function fixesMainSaid(fixes: FixesMain): string {
  return fixes.state === "fixing"
    ? `Fixing main: ${fixes.check} failed${fixes.merge === undefined ? "" : ` after #${fixes.merge}`}`
    : `Fixed main in #${fixes.fixed_in}`;
}

/** The mark's state. A `state` this build does not know draws as working, the one a Job still has. */
export function fixesMainMark(fixes: FixesMain): "fixing" | "fixed" {
  return fixes.state === "fixed" ? "fixed" : "fixing";
}
