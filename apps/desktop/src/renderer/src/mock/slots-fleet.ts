// The worktree pool Cleanup draws, one slot in each state: a Job's, a
// session's, free, two stranded, a killed Job's that kept its slot, not made yet, warm and cold, and one behind
// main. A stranded or kept one can be rescued: a Scout reads it a file at a time.

import type {
  ChangeSlotPool,
  JobSummary,
  Outcome,
  RescueSlot,
  SlotFinding,
  WorktreeSlot,
  WorktreesHeld,
} from "@armada/protocol";
import type { RescueOutcome } from "@armada/screens/src/slot-rescue";

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

/** What a rescue Scout reads of one stranded slot, a file to a poll, and what it concludes. */
type Scouted = Pick<SlotFinding, "commit" | "uncommitted" | "read" | "searched" | "summary">;

const SCOUTED: Record<number, Scouted> = {
  8: {
    commit: "b61d3a0e94",
    uncommitted: true,
    read: ["crates/fleet/src/manifest.rs", "crates/api/src/routes.rs"],
    searched: ["read_manifest in crates/"],
    summary:
      "The manifest read retried on a short answer, committed. The route and its test are edited and not committed; the test asserts a retry count the read does not return yet.",
  },
  4: {
    commit: "9d41e07b2c",
    uncommitted: true,
    read: ["src/reader/retry.rs", "src/lib.rs"],
    searched: ["retry_short_read in src/"],
    summary:
      "A retry for a short read, finished in the commit and half-moved in the working files. src/reader/retry.rs holds the new loop; src/lib.rs still calls the old one. No test covers either.",
  },
  7: {
    commit: "e08c4d1a77",
    uncommitted: true,
    read: ["crates/fleet/src/slots.rs", "crates/fleet/src/leasing.rs"],
    searched: ["LeaseRecord in crates/"],
    summary:
      "The lease record written ahead of the checkout, with the read on start still open. leasing.rs calls a record function slots.rs does not define yet.",
  },
};

/** The slot a stranded one becomes once rescued: free, with nothing of its holder left. */
function freed(one: WorktreeSlot): WorktreeSlot {
  const { branch: _b, since: _s, stranded: _t, rescue: _r, ...rest } = one;
  return { ...rest, held: { state: "free" }, warm: false };
}

/**
 * `rescue_slot` on the mock's pool, by Fleet's rules: start and stop move the
 * Scout, scrap and stash free the slot. A Scrap keeps the branch where it holds
 * commits nothing else has; a Stash commits the uncommitted work to its branch.
 */
export function rescued(
  held: WorktreesHeld,
  manifestId: string,
  rescue: RescueSlot,
): { held: WorktreesHeld; outcome: RescueOutcome } {
  const slots = held.slots ?? [];
  const target = slots.find((one) => one.manifest_id === manifestId && one.slot === rescue.slot);
  const answer = (change: WorktreeSlot, outcome: RescueOutcome) => ({
    held: { ...held, slots: slots.map((one) => (one === target ? change : one)) },
    outcome,
  });
  const refuse = (code: string, message: string) => ({ held, outcome: refusedAs(code, message) as RescueOutcome });
  if (target === undefined) return refuse("fleet.no_such_slot", `no slot-${rescue.slot}`);
  if (target.held.state === "busy") return refuse("fleet.slot_busy", "a lease is under way");
  // A Job that ended and kept its slot is rescued as a stranded one is.
  const holds = target.held.state === "stranded" || (target.held.state === "job" && target.held.kept !== undefined);
  if (!holds) return refuse("fleet.slot_not_stranded", `slot-${target.slot} is not stranded`);
  const reading = target.rescue?.state === "reading";
  const receipt = { manifest_id: manifestId, slot: target.slot };
  switch (rescue.act) {
    case "start": {
      const script = SCOUTED[target.slot];
      if (reading) return refuse("fleet.rescue_reading", "a Scout is reading it");
      if (script === undefined) return refuse("fleet.rescue_on_no_branch", "no branch to read");
      const finding: SlotFinding = {
        state: "reading",
        commit: script.commit,
        uncommitted: script.uncommitted,
        read: [],
        searched: [],
      };
      return answer({ ...target, rescue: finding }, { ok: true, rescued: receipt });
    }
    case "stop":
      if (!reading || target.rescue === undefined) return refuse("fleet.rescue_not_running", "no Scout is reading it");
      return answer({ ...target, rescue: { ...target.rescue, state: "stopped" } }, { ok: true, rescued: receipt });
    case "scrap":
    case "stash": {
      if (reading) return refuse("fleet.rescue_reading", "a Scout is reading it");
      if (target.branch === undefined) return refuse("fleet.rescue_on_no_branch", "no branch");
      if (target.branch === target.base) return refuse("fleet.rescue_on_the_base", `on ${target.base}`);
      const { branch } = target;
      const outcome: RescueOutcome =
        rescue.act === "scrap"
          ? { ok: true, rescued: { ...receipt, branch, branch_kept: (target.stranded?.unpushed ?? 0) > 0 } }
          : { ok: true, rescued: { ...receipt, branch, committed: "c0ffee1a4d" } };
      return answer(freed(target), outcome);
    }
  }
}

/**
 * One poll of the pool while a Scout reads: each reading slot has read one more
 * file, and the last poll is its answer. Fleet does the reading on its own; the
 * mock does it when Bridge asks, which is also how Bridge learns of it.
 */
export function scoutRead(held: WorktreesHeld): WorktreesHeld {
  const slots = held.slots ?? [];
  if (!slots.some((one) => one.rescue?.state === "reading")) return held;
  return {
    ...held,
    slots: slots.map((one) => {
      const finding = one.rescue;
      const script = SCOUTED[one.slot];
      if (finding?.state !== "reading" || script === undefined) return one;
      const next = script.read[finding.read.length];
      const rescue: SlotFinding =
        next !== undefined
          ? { ...finding, read: [...finding.read, next] }
          : { ...finding, state: "answered", searched: script.searched, ...(script.summary === undefined ? {} : { summary: script.summary }) };
      return { ...one, rescue };
    }),
  };
}
