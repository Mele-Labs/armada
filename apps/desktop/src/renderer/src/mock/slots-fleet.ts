// The worktree pool Cleanup draws, one slot in each state: a Job's, a
// session's, free, stranded, not made yet, warm and cold, and one behind main.

import type { ChangeSlotPool, JobSummary, Outcome, WorktreeSlot, WorktreesHeld } from "@armada/protocol";

const AT = "/Users/user/armada/.armada/slots";

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
      }),
      slot(5, { held: { state: "free" }, behind: 0 }),
      slot(6, { held: { state: "unmade" } }),
    ],
  };
}

/**
 * Files written into a slot after the read, which its remove then meets: what
 * the walk's refusal shows. Slot 5 reads free and cold, and is not empty.
 */
const WRITTEN_SINCE_READ: Record<number, string> = { 5: "notes.md" };

const refusedAs = (code: string, message: string): Outcome => ({
  ok: false,
  why: "refused",
  error: { code, message, run_id: "mock", fields: {}, chain: [] },
});

/**
 * `change_slot_pool` on the mock's pool, by the pool's rules: add takes the
 * lowest number unused and is not made; remove takes the slot pressed, only a
 * free or unmade one, never the last; close and open last until undone.
 */
export function reshaped(
  held: WorktreesHeld,
  manifestId: string,
  change: ChangeSlotPool,
): { held: WorktreesHeld; outcome: Outcome } {
  const all = held.slots ?? [];
  const pool = all.filter((one) => one.manifest_id === manifestId);
  const others = all.filter((one) => one.manifest_id !== manifestId);
  const answer = (slots: WorktreeSlot[]) => ({
    held: { ...held, slots: [...others, ...slots].sort((a, b) => a.slot - b.slot) },
    outcome: { ok: true } as Outcome,
  });
  if (change.act === "add") {
    let n = 1;
    while (pool.some((one) => one.slot === n)) n += 1;
    return answer([...pool, { ...slot(n, { held: { state: "unmade" } }), manifest_id: manifestId }]);
  }
  const target = pool.find((one) => one.slot === change.slot);
  if (target === undefined) {
    return { held, outcome: refusedAs("fleet.no_such_slot", `no slot-${change.slot ?? "?"}`) };
  }
  if (change.act === "close" || change.act === "open") {
    const closed = change.act === "close";
    return answer(pool.map((one) => (one === target ? { ...one, closed } : one)));
  }
  const why = ((): Outcome | null => {
    switch (target.held.state) {
      case "job":
        return refusedAs("fleet.slot_held", `held by job ${target.held.job_id}`);
      case "session":
        return refusedAs("fleet.slot_held", `held by ${target.held.holder}`);
      case "stranded":
        return refusedAs("fleet.slot_stranded", `stranded, holding ${target.held.why}`);
      case "busy":
        return refusedAs("fleet.slot_busy", "a lease is under way");
      case "not_a_checkout":
        return refusedAs("fleet.slot_not_a_checkout", "not a checkout");
      default:
        break;
    }
    if (pool.length === 1) return refusedAs("fleet.slot_last", "the last slot");
    const written = WRITTEN_SINCE_READ[target.slot];
    return written === undefined ? null : refusedAs("fleet.slot_dirty", `1 uncommitted, first ${written}`);
  })();
  return why === null ? answer(pool.filter((one) => one !== target)) : { held, outcome: why };
}
