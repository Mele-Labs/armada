// Cleanup's grid over the pool of `slots-fleet.ts` plus the Jobs' worktrees: a
// finished Job's bay holding uncommitted files and an unmerged branch, a
// running Job's bay, and two worktrees outside the pool. Clear, Delete branch
// and Forget change what the next read answers, as Fleet's reclaim does.

import type { JobSummary, Outcome, WorktreeHeld, WorktreesHeld } from "@armada/protocol";

import { slotsHeld } from "./slots-fleet";

const ROOT = "/Users/user/armada/.armada/worktrees";
const WIP_COMMIT = "d41f8a6c20be";

export type GridJobs = {
  /** Finished, and holds the first bay: files nobody committed and a branch nothing merged. */
  finished: JobSummary;
  /** Still running, and holds the fifth bay. Nothing may reclaim it. */
  running: JobSummary;
  /** Finished, outside the pool, with files nobody committed. */
  rejected: JobSummary;
  /** Finished, outside the pool, its checkout gone and its branch left. */
  failed: JobSummary;
};

/** `GET /worktrees`: the pool of `slotsHeld` with closed and running bays, and the Jobs' worktrees. */
export function gridHeld(jobs: GridJobs, now: number): WorktreesHeld {
  const ago = (minutes: number) => new Date(now - minutes * 60_000).toISOString();
  const base = slotsHeld(jobs.finished, now);
  const slots = (base.slots ?? []).map((one) => {
    if (one.slot === 3) return { ...one, closed: true };
    // An agent session whose checkout holds files nobody committed, and a clean one.
    if (one.slot === 2) {
      return {
        ...one,
        held: { state: "session" as const, holder: "claude (pid 44698)" },
        stranded: { uncommitted: ["packages/screens/src/SlotPools.tsx", "notes/release.md"], commits: [], unpushed: 0 },
      };
    }
    if (one.slot === 6) {
      return {
        ...one,
        held: { state: "session" as const, holder: "zsh (pid 4120)" },
        branch: "fleet/slot-lease-record",
        since: ago(42),
        warm: true,
        behind: 3,
      };
    }
    if (one.slot !== 5) return one;
    return {
      ...one,
      held: { state: "job" as const, job_id: jobs.running.id, job_title: jobs.running.title },
      branch: `armada/${jobs.running.handle}`,
      since: ago(20),
      warm: true,
      behind: 1,
    };
  });
  const at = (slot: number) => slots.find((one) => one.slot === slot)!.path;
  const row = (job: JobSummary, rest: Partial<WorktreeHeld>): WorktreeHeld => ({
    job_id: job.id,
    job_title: job.title,
    status: "completed_success",
    last_moved_at: ago(4 * 24 * 60),
    path: `${ROOT}/${job.id}`,
    branch: `armada/${job.handle}`,
    held: [],
    on_disk: true,
    ...rest,
  });
  return {
    slots,
    worktrees: [
      row(jobs.finished, {
        path: at(1),
        held: [
          { why: "uncommitted", files: ["src/reader/retry.rs", "src/reader/mod.rs", "notes/retry.md"] },
          { why: "unmerged", base: "main", commits: 2, tip: "4e9a1c07d3b8" },
        ],
      }),
      row(jobs.running, {
        path: at(5),
        status: "running",
        last_moved_at: ago(5),
        held: [{ why: "not_terminal", status: "running" }],
      }),
      row(jobs.rejected, {
        status: "rejected",
        held: [
          { why: "uncommitted", files: ["crates/api/src/routes.rs"] },
          { why: "unmerged", base: "main", commits: 1, tip: "b3d70e2a915c" },
        ],
      }),
      row(jobs.failed, {
        status: "completed_failed",
        on_disk: false,
        held: [{ why: "unmerged", base: "main", commits: 4, tip: "91c5f0aa27e6" }],
      }),
    ],
  };
}

/**
 * `reclaim_worktree` on the mock's read: uncommitted files are committed to the
 * branch, the checkout goes, and the branch stays. `undefined` where the read holds no worktree
 * of that Job, which is a scenario the fake answers on its own.
 */
export function reclaimedIn(
  held: WorktreesHeld,
  jobId: string,
): { held: WorktreesHeld; outcome: Outcome } | undefined {
  const target = held.worktrees.find((one) => one.job_id === jobId);
  if (target === undefined) return undefined;
  if (target.held.some((reason) => reason.why === "not_terminal")) {
    return { held, outcome: refusedAs("fleet.not_reclaimable", "the Job has not ended") };
  }
  const unmerged = target.held.find((reason) => reason.why === "unmerged");
  const bay = (held.slots ?? []).find((one) => one.held.state === "job" && one.held.job_id === jobId);
  // Uncommitted files are committed to the branch, which then holds one more commit.
  const dirty = target.held.flatMap((reason) => (reason.why === "uncommitted" ? reason.files : []));
  const saved = dirty.length === 0 ? undefined : { commit: WIP_COMMIT, files: dirty };
  const standing =
    saved === undefined
      ? unmerged
      : {
          why: "unmerged" as const,
          base: unmerged?.base ?? "main",
          commits: (unmerged?.commits ?? 0) + 1,
          tip: WIP_COMMIT,
        };
  const after: WorktreeHeld = {
    ...target,
    on_disk: false,
    ...(bay === undefined ? {} : { path: `${ROOT}/${jobId}` }),
    held: standing === undefined ? [] : [standing],
  };
  if (bay !== undefined) {
    const { branch: _b, since: _s, ...rest } = bay;
    const freed = { ...rest, held: { state: "free" as const } };
    held = { ...held, slots: (held.slots ?? []).map((one) => (one === bay ? freed : one)) };
  }
  return {
    held: { ...held, worktrees: held.worktrees.map((one) => (one === target ? after : one)) },
    outcome: {
      ok: true,
      reclaimed: {
        job_id: jobId,
        worktree: { path: target.path, removed: true },
        branch: {
          branch: target.branch,
          deleted: false,
          ...(standing === undefined
            ? {}
            : { unmerged_commits: standing.commits, tip: standing.tip, base: standing.base }),
        },
        ...(saved === undefined ? {} : { saved }),
      },
    },
  };
}

/** `delete_branch` on the mock's read: the unmerged reason goes, and the tip is what comes back. */
export function branchDeletedIn(
  held: WorktreesHeld,
  jobId: string,
  tip: string,
): { held: WorktreesHeld; outcome: Outcome } | undefined {
  const target = held.worktrees.find((one) => one.job_id === jobId);
  if (target === undefined) return undefined;
  const after: WorktreeHeld = { ...target, held: target.held.filter((reason) => reason.why !== "unmerged") };
  return {
    held: { ...held, worktrees: held.worktrees.map((one) => (one === target ? after : one)) },
    outcome: { ok: true, branchDeleted: { job_id: jobId, branch: target.branch, tip } },
  };
}

/** `forget_job` on the mock's read: the row goes. */
export function forgottenIn(held: WorktreesHeld, jobId: string): WorktreesHeld {
  return { ...held, worktrees: held.worktrees.filter((one) => one.job_id !== jobId) };
}

const refusedAs = (code: string, message: string): Outcome => ({
  ok: false,
  why: "refused",
  error: { code, message, run_id: "mock", fields: {}, chain: [] },
});
