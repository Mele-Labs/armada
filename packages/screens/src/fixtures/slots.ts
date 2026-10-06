// The worktree pool a mock Fleet answers `GET /worktrees` with. Shared by the Cleanup and Slots
// fleets, so neither imports the other.

import type { JobSummary, WorktreeSlot, WorktreesHeld } from "@armada/protocol";

const AT = "/Users/user/armada/.armada/slots";

export function slot(n: number, rest: Partial<WorktreeSlot> & Pick<WorktreeSlot, "held">): WorktreeSlot {
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
        held: { state: "session", holder: "zsh (pid 4120)" },
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
        stranded: {
          uncommitted: ["src/lib.rs", "src/reader/retry.rs"],
          commits: [
            { sha: "9d41e07b2c", subject: "Retry a short read once", home: "only_here" },
            { sha: "3b7a1c9e55", subject: "Split the reader from the parser", home: "on_remote" },
          ],
          unpushed: 1,
        },
      }),
      slot(5, { held: { state: "free" }, behind: 0 }),
      slot(6, { held: { state: "unmade" } }),
      slot(7, {
        held: { state: "stranded", why: "3 uncommitted, first crates/fleet/src/slots.rs" },
        branch: "fleet/slot-lease-record",
        since: ago(5 * 24 * 60),
        behind: 12,
        stranded: {
          uncommitted: ["crates/fleet/src/slots.rs", "crates/fleet/src/leasing.rs", "notes/lease.md"],
          commits: [
            { sha: "e08c4d1a77", subject: "Write the lease record before the checkout", home: "only_here" },
            { sha: "71f29b3d08", subject: "Name the holder in the record", home: "only_here" },
            { sha: "c5a6e0f912", subject: "Read the record on start", home: "on_remote" },
          ],
          unpushed: 2,
        },
      }),
      // A killed Job whose release was refused: its work is still in the slot.
      slot(8, {
        held: {
          state: "job",
          job_id: "01KEPTJOB",
          job_title: "Retry the manifest read",
          job_status: "killed",
          kept: "5 uncommitted, first crates/api/src/routes.rs",
        },
        branch: "armada/14-retry-the-manifest-read",
        since: ago(26 * 60),
        behind: 3,
        stranded: {
          uncommitted: [
            "crates/api/src/routes.rs",
            "crates/api/src/routes/served.rs",
            "crates/fleet/src/manifest.rs",
            "crates/fleet/src/tests/manifest.rs",
            "docs/notes/retry.md",
          ],
          commits: [
            { sha: "b61d3a0e94", subject: "Retry the manifest read on a short answer", home: "only_here" },
            { sha: "28c7f5d1a3", subject: "Name the manifest in the read error", home: "only_here" },
            { sha: "0f4e8b2c61", subject: "Bump the retry limit", home: "on_main" },
          ],
          unpushed: 2,
        },
      }),
    ],
  };
}

