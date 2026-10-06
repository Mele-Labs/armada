// What a tile offers for a held worktree, and what its confirm and its
// receipts say.
//
// **Unit tests and not a `play`**, because every function here computes: a
// browser mounting a screen to check a rule is a unit test paying a browser's
// price. What earns a `play` is behaviour a rendering cannot show, and that is
// in `Worktrees.test.tsx` beside it.
//
// The rule this file exists for is that nothing may reclaim a Job that has not
// ended, because Fleet refuses it with `fleet.not_reclaimable`, and that Forget
// is withheld while a checkout or a branch still stands.

import { expect, test } from "vitest";
import type { JobSummary, WorktreeHeld, WorktreeReclaimed } from "@armada/protocol";

import { branchDeletedSaid, costOf, namedByHandle, offeredActs, reclaimedSaid, sitting, slotNameOf, unmergedOf } from "./held";

/** The shape fleet answers with, in one place so no case drifts from it. */
function held(over: Partial<WorktreeHeld> = {}): WorktreeHeld {
  return {
    job_id: "01JOB0001",
    job_title: "Port the settings selectors",
    status: "completed_success",
    last_moved_at: "2026-08-30T09:14:00Z",
    path: "/Users/user/armada/.armada/worktrees/01JOB0001",
    branch: "armada/01JOB0001",
    held: [],
    on_disk: true,
    ...over,
  };
}

const UNMERGED = {
  why: "unmerged",
  base: "main",
  commits: 3,
  tip: "9f1c2ab84d5e",
} as const;

const NOW = Date.parse("2026-09-03T12:00:00Z");

test("an age is coarse, and reads in the unit a person decides in", () => {
  expect(sitting("2026-09-03T11:59:40Z", NOW)).toBe("under a minute");
  expect(sitting("2026-09-03T11:59:00Z", NOW)).toBe("1 minute");
  expect(sitting("2026-09-03T11:38:00Z", NOW)).toBe("22 minutes");
  expect(sitting("2026-09-03T11:00:00Z", NOW)).toBe("1 hour");
  expect(sitting("2026-09-03T04:00:00Z", NOW)).toBe("8 hours");
  expect(sitting("2026-09-02T12:00:00Z", NOW)).toBe("1 day");
  expect(sitting("2026-08-30T09:14:00Z", NOW)).toBe("4 days");
});

/**
 * It rounds down and never up. `last_moved_at` is already a floor — armada
 * moved the job then, and the files were written at or before it — so rounding
 * up would turn a floor into a claim.
 */
test("an age rounds down, because the stamp it is measured from is a floor", () => {
  // Twenty-three and a half hours is not "1 day".
  expect(sitting("2026-09-02T12:30:00Z", NOW)).toBe("23 hours");
  // And three days and twenty-two hours is not "4 days".
  expect(sitting("2026-08-30T14:00:00Z", NOW)).toBe("3 days");
});

/**
 * A stamp that will not parse says nothing rather than showing an age measured
 * from zero — the convention `instant` sets. A stamp in the future is the same
 * refusal: a clock disagreeing is not an age.
 */
test("an unreadable or future stamp draws no age at all", () => {
  expect(sitting("not a date", NOW)).toBeNull();
  expect(sitting("2026-09-04T12:00:00Z", NOW)).toBeNull();
});

function job(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: "01BLOCKER00000000000000000",
    handle: "9-fix-the-retry-ceiling",
    title: "Fix the retry ceiling",
    status: "running",
    workflow_id: "bug",
    owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-08-30T09:00:00Z",
    ...over,
  };
}

test("a depended_on reason names its blocker by handle, not by id", () => {
  const row = held({ held: [{ why: "depended_on", by: ["01BLOCKER00000000000000000"] }] });

  const named = namedByHandle(row, [job()]);

  expect(named.held).toEqual([{ why: "depended_on", by: ["9-fix-the-retry-ceiling"] }]);
});

test("a blocker no longer on the board keeps its id, rather than showing nothing", () => {
  const row = held({ held: [{ why: "depended_on", by: ["01GONE00000000000000000000"] }] });

  const named = namedByHandle(row, [job()]);

  expect(named.held).toEqual([{ why: "depended_on", by: ["01GONE00000000000000000000"] }]);
});

test("every other reason passes through namedByHandle unchanged", () => {
  const row = held({ held: [{ why: "unmerged", base: "main", commits: 1, tip: "abc123" }] });

  expect(namedByHandle(row, [job()])).toEqual(row);
});

/** A Job that has not ended offers nothing: Fleet would refuse every act with a 409. */
test("a Job that has not ended is offered no act at all", () => {
  const running = held({ status: "running", held: [{ why: "not_terminal", status: "running" }, UNMERGED] });

  expect(offeredActs(running)).toEqual({ clear: false, deleteBranch: false, forget: false });
});

test("Clear is offered only while the checkout is on disk", () => {
  expect(offeredActs(held({ on_disk: true, held: [UNMERGED] })).clear).toBe(true);
  expect(offeredActs(held({ on_disk: false, held: [UNMERGED] })).clear).toBe(false);
});

test("Delete branch is offered only while an unmerged reason is present", () => {
  expect(offeredActs(held({ on_disk: false, held: [UNMERGED] })).deleteBranch).toBe(true);
  expect(offeredActs(held({ on_disk: false, held: [] })).deleteBranch).toBe(false);
});

/** Fleet answers a branch delete with a 409 while the worktree is on disk, so the act is not offered then. */
test("Delete branch waits for the worktree to go", () => {
  expect(offeredActs(held({ on_disk: true, held: [UNMERGED] })).deleteBranch).toBe(false);
});

/**
 * **The case Forget exists to guard.** Forgetting a record whose checkout
 * still stands orphans that disk from this page, because `worktrees_held` walks
 * Job records and not directories.
 */
test("Forget is withheld while the checkout, an unmerged branch or an unanswered base stands", () => {
  expect(offeredActs(held({ on_disk: true, held: [] })).forget).toBe(false);
  expect(offeredActs(held({ on_disk: false, held: [UNMERGED] })).forget).toBe(false);
  expect(offeredActs(held({ on_disk: false, held: [{ why: "base_unanswered", detail: "none of main is here" }] })).forget).toBe(
    false,
  );
});

test("a worktree with neither checkout nor branch left offers Forget alone", () => {
  expect(offeredActs(held({ on_disk: false, held: [] }))).toEqual({ clear: false, deleteBranch: false, forget: true });
});

/** The cost of a Clear separates what ends from what survives. */
test("a Clear's cost names the uncommitted files and the unmerged branch it keeps", () => {
  const row = held({ held: [{ why: "uncommitted", files: ["src/log.rs", "notes.md"] }, UNMERGED] });

  expect(costOf(row)).toEqual({
    files: ["src/log.rs", "notes.md"],
    branch: { name: "armada/01JOB0001", commits: 3, tip: "9f1c2ab84d5e", base: "main" },
  });
});

test("a Clear that ends nothing names no file and no branch", () => {
  expect(costOf(held({ held: [] }))).toEqual({ files: [] });
});

test("the tip a branch delete sends is the unmerged reason's own", () => {
  expect(unmergedOf(held({ held: [UNMERGED] }))?.tip).toBe("9f1c2ab84d5e");
  expect(unmergedOf(held({ held: [] }))).toBeNull();
});

function reclaimed(over: Partial<WorktreeReclaimed> = {}): WorktreeReclaimed {
  return {
    job_id: "01JOB0001",
    worktree: { path: "/p", removed: true },
    branch: { branch: "armada/01JOB0001", deleted: false, unmerged_commits: 3 },
    ...over,
  };
}

test("a receipt says each half, the checkout then the branch, as bare facts", () => {
  expect(reclaimedSaid(reclaimed())).toEqual(["Worktree removed", "Branch kept: 3 commits not on main"]);
  expect(reclaimedSaid(reclaimed({ branch: { branch: "b", deleted: false, unmerged_commits: 1, base: "develop" } }))[1]).toBe(
    "Branch kept: 1 commit not on develop",
  );
});

/** The receipt for a Clear that committed uncommitted files says only where they went and what was freed. */
test("a receipt for committed files names the branch and what was freed", () => {
  const saved = { commit: "d41f8a6c20be", files: ["src/log.rs"] };
  expect(reclaimedSaid(reclaimed({ saved }), true)).toEqual(["Committed to armada/01JOB0001, slot released"]);
  expect(reclaimedSaid(reclaimed({ saved }))).toEqual(["Committed to armada/01JOB0001, worktree removed"]);
  expect(reclaimedSaid(reclaimed({ saved: null }), true)[0]).toBe("Slot released, worktree kept");
});

test("a bay is told from a worktree outside the pool by its path", () => {
  expect(slotNameOf("/r/.armada/slots/slot-4")).toBe("slot-4");
  expect(slotNameOf("/r/.armada/worktrees/01JOB")).toBeNull();
});

test("a locked worktree answers ok and the receipt says it is kept", () => {
  expect(reclaimedSaid(reclaimed({ worktree: { path: "/p", removed: false, why: "locked by a Pilot" } }))[0]).toBe(
    "Worktree kept: locked by a Pilot",
  );
});

/** A bay is released to the pool and its directory stays, so it is never said to be removed. */
test("a bay's receipt says the slot was released, not removed", () => {
  expect(reclaimedSaid(reclaimed(), true)[0]).toBe("Slot released, worktree kept");
});

test("a deleted branch names its tip, and one that could not be deleted says why", () => {
  expect(reclaimedSaid(reclaimed({ branch: { branch: "b", deleted: true, tip: "abc123" } }))[1]).toBe("Branch deleted at abc123");
  expect(reclaimedSaid(reclaimed({ branch: { branch: "b", deleted: false, why: "checked out elsewhere" } }))[1]).toBe(
    "Branch kept: checked out elsewhere",
  );
});

test("a branch delete's receipt names the tip its commits are reachable from", () => {
  expect(branchDeletedSaid({ job_id: "x", branch: "b", tip: "9f1c2ab84d5e" })).toEqual(["Branch deleted at 9f1c2ab84d5e"]);
});
