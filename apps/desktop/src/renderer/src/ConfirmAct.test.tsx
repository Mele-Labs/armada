// The Job's own Clear confirm: what it commits and frees, drawn from the held
// worktrees the Cleanup grid reads, so both say the same thing.
//
// Here rather than in `packages/screens` because the confirm is the app's, and
// the stylesheet that sizes it is the app's too.

import type { HeldWorktrees, WorktreeHeld } from "@armada/protocol";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import "./styles/index.css";
import { ConfirmAct } from "./ConfirmAct";

const roots: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

function worktree(over: Partial<WorktreeHeld>): WorktreeHeld {
  return {
    job_id: "01JOB",
    job_title: "Fix the reader",
    status: "completed_failed",
    last_moved_at: "2026-10-01T09:00:00Z",
    path: "/r/.armada/slots/slot-3",
    branch: "armada/3-fix-the-reader",
    on_disk: true,
    held: [{ why: "uncommitted", files: ["src/reader.rs", "notes.md"] }],
    ...over,
  };
}

function draw(held: HeldWorktrees, onWant = vi.fn(), cleanupOpen = false): () => void {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  root.render(
    <ConfirmAct
      confirming={{ act: "reclaim_worktree", jobId: "01JOB" }}
      held={held}
      jobs={[]}
      onWant={onWant}
      cleanupOpen={cleanupOpen}
      restartNote=""
      onRestartNote={() => {}}
      onCancel={() => {}}
      onConfirm={() => {}}
    />,
  );
  return () => root.unmount();
}

const read = (...worktrees: WorktreeHeld[]): HeldWorktrees => ({ state: "read", held: { worktrees } });

test("Clear confirm: a bay holding uncommitted files says it commits them and releases the slot", async () => {
  draw(read(worktree({})));
  const confirm = page.getByRole("dialog");
  await expect.element(confirm.getByText("Commits the uncommitted files to branch armada/3-fix-the-reader as a WIP commit")).toBeVisible();
  await expect.element(confirm.getByRole("list", { name: "Uncommitted files" })).toHaveTextContent("src/reader.rs");
  await expect.element(confirm.getByText("Releases slot-3")).toBeVisible();
  await expect.element(confirm.getByText("Keeps branch armada/3-fix-the-reader")).toBeVisible();
  expect(confirm.getByText(/Refused|Deletes uncommitted/).elements()).toHaveLength(0);
});

test("Clear confirm: a worktree outside the pool says it commits first and then removes it", async () => {
  draw(read(worktree({ path: "/r/.armada/worktrees/01JOB" })));
  const confirm = page.getByRole("dialog");
  await expect.element(confirm.getByText("Commits the uncommitted files to branch armada/3-fix-the-reader as a WIP commit")).toBeVisible();
  await expect.element(confirm.getByText("Removes the worktree at /r/.armada/worktrees/01JOB")).toBeVisible();
});

test("Clear confirm: a clean worktree keeps the plain confirm", async () => {
  draw(read(worktree({ held: [] })));
  const confirm = page.getByRole("dialog");
  await expect.element(confirm.getByText(/Uncommitted files are committed to its branch first/)).toBeVisible();
  expect(confirm.getByText(/^Commits the uncommitted files/).elements()).toHaveLength(0);
});

test("Clear confirm: the held read is asked for while the confirm is up, and left as Cleanup had it after", async () => {
  const onWant = vi.fn();
  const undraw = draw({ state: "none" }, onWant, false);
  await vi.waitFor(() => expect(onWant).toHaveBeenLastCalledWith(true));
  undraw();
  await vi.waitFor(() => expect(onWant).toHaveBeenLastCalledWith(false));

  // Cleanup has the read open under it, so closing the confirm must not close it.
  const again = vi.fn();
  const over = draw({ state: "none" }, again, true);
  await vi.waitFor(() => expect(again).toHaveBeenLastCalledWith(true));
  over();
  await vi.waitFor(() => expect(again).toHaveBeenLastCalledWith(true));
});
