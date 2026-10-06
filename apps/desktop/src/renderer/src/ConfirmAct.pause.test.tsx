// The Pause confirm and the Resume confirm the app opens, over the Board's rows
// and the held read: the facts a pause lists, that a refusal is said inside the
// confirm that was pressed, and that Resume says it does not send the act that
// opened it.

import type { HeldWorktrees, JobSummary, WorktreeHeld } from "@armada/protocol";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import "./styles/index.css";
import { ConfirmAct, type Confirming } from "./ConfirmAct";

const roots: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

const job = (over: Partial<JobSummary> = {}): JobSummary => ({
  id: "01JOB",
  handle: "2-debounce",
  title: "Debounce the Job Board",
  status: "running",
  workflow_id: "bug",
  owner_manifest_id: "armada",
  origin: "dispatched",
  urgency: "normal",
  atomic: false,
  model: "sonnet",
  created_at: "2026-10-06T09:00:00Z",
  branch: "armada/2-debounce",
  ...over,
});

const worktree: WorktreeHeld = {
  job_id: "01JOB",
  job_title: "Debounce the Job Board",
  status: "running",
  last_moved_at: "2026-10-06T09:50:00Z",
  path: "/r/.armada/slots/slot-3",
  branch: "armada/2-debounce",
  on_disk: true,
  held: [{ why: "uncommitted", files: ["src/a.rs"] }],
};

function draw(confirming: Confirming, summary: JobSummary, refused?: string) {
  const onConfirm = vi.fn();
  const onCancel = vi.fn();
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  const held: HeldWorktrees = { state: "read", held: { worktrees: [worktree] } };
  root.render(
    <ConfirmAct
      confirming={confirming}
      held={held}
      jobs={[summary]}
      refused={refused}
      onWant={() => {}}
      cleanupOpen={false}
      restartNote=""
      onRestartNote={() => {}}
      onCancel={onCancel}
      onConfirm={onConfirm}
    />,
  );
  return { onConfirm, onCancel };
}

test("Pause on a running Job lists what it stops, commits and releases, and the step that restarts", async () => {
  const { onConfirm } = draw({ act: "pause_job", jobId: "01JOB" }, job());
  const confirm = page.getByRole("dialog", { name: "Pause this job?" });
  await expect.element(confirm.getByText("Stops the Drone and its processes")).toBeVisible();
  await expect.element(confirm.getByText("Commits uncommitted files to branch armada/2-debounce as a WIP commit")).toBeVisible();
  await expect.element(confirm.getByText("Releases slot-3")).toBeVisible();
  await expect.element(confirm.getByText("The step restarts on Resume")).toBeVisible();
  await userEvent.click(confirm.getByRole("button", { name: "Pause" }));
  expect(onConfirm).toHaveBeenCalledOnce();
});

test("a refusal is said inside the Pause confirm, which stays", async () => {
  draw({ act: "pause_job", jobId: "01JOB" }, job(), "Not paused: its Checks are reading the worktree");
  await expect.element(page.getByRole("dialog", { name: "Pause this job?" }).getByRole("alert")).toHaveTextContent(
    "Not paused: its Checks are reading the worktree",
  );
});

test("the Resume confirm an act opened says the act is not sent, and Cancel sends nothing", async () => {
  const { onConfirm, onCancel } = draw({ act: "resume_job", jobId: "01JOB", pressed: true }, job({ paused: { by: "person", at: "2026-10-06T09:56:00Z", resuming: false } }));
  const confirm = page.getByRole("dialog", { name: "This job is paused" });
  await expect.element(confirm.getByText("Does not send the act you pressed")).toBeVisible();
  expect(confirm.getByText(/slot is free|waiting for a slot|full/i).elements()).toHaveLength(0);
  await userEvent.click(confirm.getByRole("button", { name: "Cancel" }));
  expect(onCancel).toHaveBeenCalledOnce();
  expect(onConfirm).not.toHaveBeenCalled();
});
