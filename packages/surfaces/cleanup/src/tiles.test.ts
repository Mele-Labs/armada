// Which worktrees are a bay's and which stand outside the pool.

import { expect, test } from "vitest";
import type { WorktreeHeld, WorktreeSlot } from "@armada/protocol";

import { joined } from "./tiles";

function slot(n: number, rest: Partial<WorktreeSlot> = {}): WorktreeSlot {
  return { manifest_id: "armada", slot: n, path: `/slots/slot-${n}`, base: "main", warm: false, held: { state: "free" }, ...rest };
}

function worktree(jobId: string, over: Partial<WorktreeHeld> = {}): WorktreeHeld {
  return {
    job_id: jobId,
    job_title: jobId,
    status: "completed_success",
    last_moved_at: "2026-08-30T09:14:00Z",
    path: `/worktrees/${jobId}`,
    branch: `armada/${jobId}`,
    held: [],
    on_disk: true,
    ...over,
  };
}

test("a bay joins its Job's worktree by job id", () => {
  const found = joined([slot(1, { held: { state: "job", job_id: "A" } })], [worktree("A"), worktree("B")]);

  expect(found.bays[0]!.held?.job_id).toBe("A");
  expect(found.outside.map((one) => one.job_id)).toEqual(["B"]);
});

test("a worktree no slot holds is outside the pool, in Fleet's order", () => {
  const found = joined([slot(1)], [worktree("B"), worktree("A")]);

  expect(found.bays[0]!.held).toBeUndefined();
  expect(found.outside.map((one) => one.job_id)).toEqual(["B", "A"]);
});

test("a worktree at a slot's path belongs to that bay even where its Job no longer holds it", () => {
  const found = joined([slot(2)], [worktree("A", { path: "/slots/slot-2" })]);

  expect(found.bays[0]!.held?.job_id).toBe("A");
  expect(found.outside).toEqual([]);
});

test("the Job's own slot wins over another worktree at its path", () => {
  const found = joined(
    [slot(1, { held: { state: "job", job_id: "A" } })],
    [worktree("B", { path: "/slots/slot-1" }), worktree("A")],
  );

  expect(found.bays[0]!.held?.job_id).toBe("A");
  expect(found.outside.map((one) => one.job_id)).toEqual(["B"]);
});
