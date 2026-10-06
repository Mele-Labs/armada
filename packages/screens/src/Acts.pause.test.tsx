// When the Job header offers Pause and Resume, and what each press asks for.
// Pause sits behind the caret so a working Job's held kill stays its face;
// Resume is the face of a paused Job, since it is the one act that gets it going.

import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import type { JobSummary } from "@armada/protocol";
import { Acts } from "./Acts";
import { motion, mount, unmount } from "./mounted";

afterEach(unmount);
beforeEach(motion);

function job(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: "01JOB",
    handle: "12-a-job",
    title: "Coalesce concurrent token refreshes",
    status: "running",
    workflow_id: "bug",
    owner_manifest_id: "01MAN",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-08-31T09:00:00Z",
    assigned_drone: "01DRONE",
    ...over,
  };
}

const PAUSED = { by: "person", at: "2026-10-06T09:56:00Z", resuming: false };

function header(summary: JobSummary, rerunningChecks = false, onAct = vi.fn()) {
  mount(
    <Acts
      job={summary}
      whole={null}
      render="working"
      acting={false}
      approving={false}
      stale={false}
      rerunningChecks={rerunningChecks}
      onAct={onAct}
      onActHeld={() => {}}
      onApprove={() => {}}
      onReport={async () => ({ ok: true })}
      reporting={false}
      onReporting={() => {}}
      onRaiseCap={() => {}}
      raising={false}
      onRaising={() => {}}
      onRaiseTurnCap={() => {}}
      raisingTurns={false}
      onRaisingTurns={() => {}}
      onCopied={() => {}}
    />,
  );
  return onAct;
}

const MENU = "Everything else this job can do";
const PAUSE = "Pause, the work is saved on its branch";

test("a working Job is offered Pause behind the caret, and the kill stays the face", async () => {
  const onAct = header(job());
  await expect.element(page.getByRole("button", { name: "Hold to kill job" })).toBeEnabled();
  await page.getByRole("button", { name: MENU }).click();
  await page.getByRole("menuitem", { name: PAUSE }).click();
  expect(onAct).toHaveBeenCalledWith("pause_job", "01JOB");
});

test("a Job parked at a review gate is offered Pause", async () => {
  header(job({ status: "awaiting_review", assigned_drone: undefined }));
  await page.getByRole("button", { name: MENU }).click();
  await expect.element(page.getByRole("menuitem", { name: PAUSE })).toBeInTheDocument();
});

test("a paused Job is offered Resume as its face, and Pause is not offered again", async () => {
  const onAct = header(job({ status: "awaiting_review", assigned_drone: undefined, paused: PAUSED }));
  await page.getByRole("button", { name: "Resume" }).click();
  expect(onAct).toHaveBeenCalledWith("resume_job", "01JOB");
  await page.getByRole("button", { name: MENU }).click();
  await expect.element(page.getByRole("menuitem", { name: PAUSE })).not.toBeInTheDocument();
});

test("Pause is not offered while the Job's Checks run", async () => {
  header(job(), true);
  // The kill is all that is left, so there is no menu to open.
  await expect.element(page.getByRole("button", { name: "Hold to kill job" })).toBeEnabled();
  await expect.element(page.getByRole("button", { name: MENU })).not.toBeInTheDocument();
});

test("Pause is not offered on a Job that is over", async () => {
  header(job({ status: "killed", assigned_drone: undefined }));
  await expect.element(page.getByRole("button", { name: "Reclaim worktree" })).toBeInTheDocument();
  await expect.element(page.getByRole("button", { name: "Pause" })).not.toBeInTheDocument();
});
