// What a tile of Cleanup's grid offers for a held worktree, and what its
// confirmations have to say.
//
// **Its own file because it is arithmetic, and arithmetic is unit-tested.** A
// `play` that computed rather than read would be a unit test paying a browser's
// price — `docs/practices/react.md` is explicit — and every line below is read
// immediately before something is destroyed.

import type { BranchDeleted, HeldReason, JobSummary, WorktreeHeld, WorktreeReclaimed } from "@armada/protocol";
import { reclaimable } from "@armada/protocol";
import type { ClearCost, Offered } from "@armada/components";
import { instant } from "./duration";

/** The `unmerged` reason on a row, where it carries one — the source of the tip a branch delete sends. */
export function unmergedOf(held: WorktreeHeld): Extract<HeldReason, { why: "unmerged" }> | null {
  return held.held.find((reason): reason is Extract<HeldReason, { why: "unmerged" }> => reason.why === "unmerged") ?? null;
}

const NONE: Offered = { clear: false, deleteBranch: false, forget: false };

/**
 * Which of a worktree's three acts a person is offered.
 *
 * **Nothing at all while the Job has not ended**: Fleet refuses a reclaim with
 * `fleet.not_reclaimable` there, so a button would be a control whose only
 * outcome is a refusal. **Delete branch waits for the worktree to go**, which Fleet
 * refuses with a 409 while it is on disk. **Forget is gated on both**: a record
 * forgotten while its checkout or its branch stands orphans disk
 * `worktrees_held` no longer walks to, and a branch whose base nothing could
 * name offers no delete, so it holds the record too.
 */
export function offeredActs(held: WorktreeHeld): Offered {
  if (!reclaimable(held)) return NONE;
  const unmerged = unmergedOf(held) !== null;
  const unanswered = held.held.some((reason) => reason.why === "base_unanswered");
  return { clear: held.on_disk, deleteBranch: unmerged && !held.on_disk, forget: !held.on_disk && !unmerged && !unanswered };
}

/**
 * What Clear ends and what it leaves standing, for its confirm: the files
 * written and committed nowhere, which no branch carries, and the unmerged
 * branch the reclaim keeps, with the tip it is reachable from.
 */
export function costOf(held: WorktreeHeld): ClearCost {
  const files = held.held.flatMap((reason) => (reason.why === "uncommitted" ? reason.files : []));
  const unmerged = unmergedOf(held);
  return {
    files,
    ...(unmerged === null
      ? {}
      : { branch: { name: held.branch, commits: unmerged.commits, tip: unmerged.tip, base: unmerged.base } }),
  };
}

/** The bay a worktree path is, `slot-4`, or null for a worktree outside the pool. */
export function slotNameOf(path: string): string | null {
  return /\/\.armada\/slots\/(slot-\d+)\/?$/.exec(path)?.[1] ?? null;
}

/**
 * What a reclaim did, in git's words: the worktree, then the branch. **A bay is
 * released to the pool and its directory stays**, so it never says removed.
 * **Where a Clear committed uncommitted files it says only that**, with where
 * they went and what became of the worktree: the branch is kept by it.
 */
export function reclaimedSaid(got: WorktreeReclaimed, pooled = false): string[] {
  const { worktree, branch } = got;
  if (got.saved != null) {
    const freed = pooled ? "slot released" : worktree.removed ? "worktree removed" : `worktree kept: ${worktree.why ?? "no reason given"}`;
    return [`Committed to ${branch.branch}, ${freed}`];
  }
  const first = pooled
    ? "Slot released, worktree kept"
    : worktree.removed
      ? "Worktree removed"
      : `Worktree kept: ${worktree.why ?? "no reason given"}`;
  const commits = branch.unmerged_commits;
  const base = branch.base ?? "main";
  const second = branch.deleted
    ? `Branch deleted at ${branch.tip ?? "its tip"}`
    : commits == null
      ? `Branch kept: ${branch.why ?? "no reason given"}`
      : `Branch kept: ${commits === 1 ? "1 commit" : `${commits} commits`} not on ${base}`;
  return [first, second];
}

/** What a branch delete did: the branch, and the tip its commits are reachable from. */
export function branchDeletedSaid(got: BranchDeleted): string[] {
  return [`Branch deleted at ${got.tip}`];
}

/**
 * A `depended_on` reason, with its `by` read against the handle a person
 * would recognise rather than the id fleet named it by.
 *
 * **The handle where Bridge holds one, the id otherwise.** The job that is
 * still holding this worktree open is, ordinarily, still on the board — but a
 * lineage fold or a sweep between the two reads can leave it named and gone,
 * and the id is still a fact worth showing rather than nothing.
 */
export function namedByHandle(held: WorktreeHeld, jobs: readonly JobSummary[]): WorktreeHeld {
  return {
    ...held,
    held: held.held.map((reason) =>
      reason.why === "depended_on"
        ? { ...reason, by: reason.by.map((jobId) => jobs.find((job) => job.id === jobId)?.handle ?? jobId) }
        : reason,
    ),
  };
}

/**
 * How long a checkout has been sitting, in the coarsest true unit.
 *
 * **Its own formatter and not `lasting`.** That one writes a run — `4m 09s`,
 * `2h 13m` — because a job in flight is read to the second. This is an age, and
 * `97h 12m` is a number nobody converts in their head at the moment they are
 * deciding whether four days of untouched work is worth opening the directory
 * for. Two quantities, two formatters, and neither pretends to be the other.
 *
 * **It rounds down and never up.** `last_moved_at` is already a floor — armada
 * last moved the job then, and the files were written at or before it — so
 * rounding up would turn a floor into a claim.
 *
 * `null` where the stamp will not parse or is in the future, which is the same
 * convention `instant` sets: a checkout whose date is unreadable says nothing
 * rather than showing an age measured from zero.
 */
export function sitting(at: string, now: number): string | null {
  const moved = instant(at);
  if (moved === null || moved > now) return null;
  const minutes = Math.floor((now - moved) / 60_000);
  if (minutes < 1) return "under a minute";
  if (minutes < 60) return minutes === 1 ? "1 minute" : `${minutes} minutes`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return hours === 1 ? "1 hour" : `${hours} hours`;
  const days = Math.floor(hours / 24);
  return days === 1 ? "1 day" : `${days} days`;
}
