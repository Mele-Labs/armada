// The worktree pool Cleanup draws, one slot in each state: a Job's, a
// session's, free, two stranded, a killed Job's that kept its slot, not made yet, warm and cold, and one behind
// main. A stranded or kept one can be rescued: a Scout reads it a file at a time.

import type {
  ChangeSlotPool,
  Outcome,
  RescueSlot,
  SlotFinding,
  WorktreeSlot,
  WorktreesHeld,
} from "@armada/protocol";
import type { RescueOutcome } from "@armada/screens/src/slot-rescue";
import { slot, slotsHeld } from "@armada/screens/src/fixtures/slots";

export { slotsHeld };

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
  if (change.act === "release") {
    if (target.held.state !== "session" || target.held.holder !== change.holder) {
      const now = target.held.state === "session" ? target.held.holder : "someone else";
      return { held, outcome: refusedAs("fleet.slot_holder_changed", `it is held by ${now} now, not by the holder that was shown`) };
    }
    const files = target.stranded?.uncommitted ?? [];
    const { stranded: _s, branch: _b, since: _since, ...rest } = target;
    const released = { ...rest, held: { state: "free" as const } };
    return {
      ...answer(pool.map((one) => (one === target ? released : one))),
      outcome: {
        ok: true,
        slotChanged: {
          manifest_id: manifestId,
          slot: target.slot,
          released: {
            branch: target.branch ?? "",
            ...(files.length === 0 ? {} : { saved: { commit: "d41f8a6c20be", files } }),
          },
        },
      },
    };
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
type Scouted = Pick<SlotFinding, "commit" | "uncommitted" | "read" | "searched" | "verdict" | "items">;

const SCOUTED: Record<number, Scouted> = {
  8: {
    commit: "b61d3a0e94",
    uncommitted: true,
    read: ["crates/api/src/routes.rs"],
    searched: ["read_manifest in crates/"],
    verdict: "unfinished",
    items: [
      "crates/api/src/routes.rs is edited and not committed",
      "The route test asserts a retry count the read does not return",
    ],
  },
  4: {
    commit: "9d41e07b2c",
    uncommitted: true,
    read: ["src/reader/retry.rs", "src/lib.rs"],
    searched: ["retry_short_read in src/"],
    verdict: "unfinished",
    items: ["src/lib.rs still calls the old read loop", "src/reader/retry.rs has no test"],
  },
  7: {
    commit: "e08c4d1a77",
    uncommitted: true,
    read: ["crates/fleet/src/slots.rs", "crates/fleet/src/leasing.rs"],
    searched: ["LeaseRecord in crates/"],
    verdict: "scraps",
    items: ["A draft note in notes/lease.md and a renamed local in slots.rs"],
  },
};

/** The slot a stranded one becomes once rescued: free, with nothing of its holder left. */
function freed(one: WorktreeSlot): WorktreeSlot {
  const { branch: _b, since: _s, stranded: _t, rescue: _r, ...rest } = one;
  return { ...rest, held: { state: "free" }, warm: false };
}

/**
 * `rescue_slot` on the mock's pool, by Fleet's rules: start and stop move the
 * Scout, scrap, stash and pick up free the slot. A Scrap keeps the branch where
 * it holds commits nothing else has; a Stash commits the uncommitted work to its
 * branch; a Pick up does that and answers the request Fleet proposes a Job from.
 */
export function rescued(
  held: WorktreesHeld,
  manifestId: string,
  rescue: RescueSlot,
): { held: WorktreesHeld; outcome: RescueOutcome; proposed?: string } {
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
    case "stash":
    case "pick_up": {
      if (reading) return refuse("fleet.rescue_reading", "a Scout is reading it");
      const found = target.rescue;
      if (rescue.act === "pick_up") {
        if (found === undefined || ((found.items ?? []).length === 0 && found.summary === undefined)) {
          return refuse("fleet.rescue_unread", `no Finding of slot-${target.slot} to continue from`);
        }
        if (found.verdict === "scraps") {
          return refuse("fleet.rescue_nothing_left", `the Finding on slot-${target.slot} says nothing is left to do`);
        }
      }
      if (target.branch === undefined) return refuse("fleet.rescue_on_no_branch", "no branch");
      if (target.branch === target.base) return refuse("fleet.rescue_on_the_base", `on ${target.base}`);
      const { branch } = target;
      const outcome: RescueOutcome =
        rescue.act === "scrap"
          ? { ok: true, rescued: { ...receipt, branch, branch_kept: (target.stranded?.unpushed ?? 0) > 0 } }
          : { ok: true, rescued: { ...receipt, branch, committed: "c0ffee1a4d" } };
      const freedSlot = answer(freed(target), outcome);
      if (rescue.act !== "pick_up") return freedSlot;
      const left = (target.rescue?.items ?? []).map((one) => `\n- ${one}`).join("");
      return { ...freedSlot, proposed: `Continue the work on branch ${branch}.${left === "" ? "" : `\n\nLeft to do:${left}`}` };
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
          : { ...finding, state: "answered", searched: script.searched, verdict: script.verdict, items: script.items };
      return { ...one, rescue };
    }),
  };
}
