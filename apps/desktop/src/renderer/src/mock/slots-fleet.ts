// The worktree pool Cleanup draws, one slot in each state: a Job's, a
// session's, free, stranded, not made yet, warm and cold, and one behind main.

import type { JobSummary, WorktreeSlot, WorktreesHeld } from "@armada/protocol";

const AT = "/Users/someone/armada/.armada/slots";

function slot(n: number, rest: Partial<WorktreeSlot> & Pick<WorktreeSlot, "held">): WorktreeSlot {
  return { manifest_id: "armada", slot: n, path: `${AT}/slot-${n}`, base: "main", warm: false, ...rest };
}

/** `GET /worktrees` over `job`, which holds the first slot. Nothing held for a person to decide. */
export function slotsHeld(job: JobSummary, now: number): WorktreesHeld {
  const ago = (minutes: number) => new Date(now - minutes * 60_000).toISOString();
  return {
    worktrees: [],
    slots: [
      slot(1, {
        held: { state: "job", job_id: job.id, job_title: job.title },
        branch: `armada/${job.handle}`,
        since: ago(130),
        warm: true,
        behind: 0,
      }),
      slot(2, {
        held: { state: "session", holder: "claude (pid 4120)" },
        branch: "fleet/slot-pool-in-cleanup",
        since: ago(42),
        warm: true,
        behind: 7,
      }),
      slot(3, { held: { state: "free" }, warm: true, behind: 2 }),
      slot(4, {
        held: { state: "stranded", why: "2 uncommitted, first src/lib.rs" },
        branch: "fleet/an-old-try",
        since: ago(3 * 24 * 60),
        behind: 31,
      }),
      slot(5, { held: { state: "free" }, behind: 0 }),
      slot(6, { held: { state: "unmade" } }),
    ],
  };
}
