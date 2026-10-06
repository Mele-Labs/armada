// Job detail, through `App`: one Job open, the way a pressed notification opens
// it, and what its answers send to Fleet. Moved here from the `Screens/Job
// detail` stories' status, overview, frozen, compose, follow and gaming groups,
// which drew the screen from `JobDetailFrom` rather than from `App` — #1224.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import { reviewAtDelivery, queued, retryingCheckFailure, running } from "@armada/jobs/fixtures/build/index";
import { JOB_ID } from "@armada/screens/src/fixtures/build/base";
import { recorded } from "@armada/screens/src/fixtures/recorded";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import {
  brokenOnMain,
  FIX_JOB,
  FIX_TITLE,
  heldByTheGamingCheck,
  reviewAtAQuestion,
  withBreakages,
  withRow,
} from "@armada/jobs/fake";
import type { } from "./scenario";
import { onJob, scenarioNamed } from "./scenario";
import { entered, mount, unmountAfterEach } from "./testing";

unmountAfterEach();

/** App, with this Job open, and the fake it talks to. */
async function opened(fixture: JobFixture, options: { whereOpen?: boolean } = {}) {
  const app = mount(onJob(fixture, options));
  await expect.element(page.getByRole("button", { name: fixture.job.handle })).toBeVisible();
  return app.api;
}

test("a judge question at the gate: Disagree, just this step, sends that answer", async () => {
  const api = await opened(reviewAtAQuestion());
  const answerJudge = vi.spyOn(api, "answerJudge");
  await page.getByRole("button", { name: "Disagree, just this step" }).click();
  expect(answerJudge).toHaveBeenCalledWith(JOB_ID, "2026-09-10T14:29:40Z", "disagree_once", undefined);
});

test("Merge asks first, with Cancel holding focus", async () => {
  await opened(reviewAtDelivery());
  await page.getByRole("button", { name: /^Merge(?! line)/ }).first().click();
  await expect.element(page.getByRole("dialog").getByRole("button", { name: "Cancel" })).toHaveFocus();
});

// The owner's arrangement of 30 Sep 2026: two split buttons, Approve and
// Reject each behind a caret. A story proves the control; only the screen
// proves the act reaches Fleet from the caret it was chosen in.
test("Approve, chosen behind Merge's caret, is what reaches Fleet", async () => {
  const api = await opened(reviewAtDelivery());
  const approveReview = vi.spyOn(api, "approveReview");
  await page.getByRole("button", { name: "The other way to take this work" }).click();
  await page.getByRole("menuitem", { name: "Approve the work" }).click();
  expect(approveReview).toHaveBeenCalledWith(JOB_ID);
});

test("Reject, behind the other caret, still asks before it ends the Job", async () => {
  const api = await opened(reviewAtDelivery());
  const rejectWork = vi.spyOn(api, "rejectWork");
  await page.getByRole("button", { name: "The other way to end this review" }).click();
  await page.getByRole("menuitem", { name: "Reject the work" }).click();
  const confirm = page.getByRole("dialog");
  await entered(confirm);
  expect(rejectWork, "the choice rejected").not.toHaveBeenCalled();
  await confirm.getByRole("button", { name: "Reject the work" }).click();
  expect(rejectWork).toHaveBeenCalledWith(JOB_ID);
});

// **The record is open at the gate**, with nothing to unfold: the owner took
// the fold away on 2 Oct 2026 (#1680, recorded in
// `2026-09-29-the-review-gate-sits-under-the-lead.md`).
test("the Job's record is open under the lead, with the decision under it", async () => {
  await opened(reviewAtDelivery());
  await expect.element(page.getByRole("region", { name: "What proves it" })).toBeVisible();
  expect(page.getByRole("button", { name: "The Job's record" }).query()).toBeNull();
  await expect.element(page.getByRole("textbox", { name: "Notes" })).toBeVisible();
});

test("picking comments on the pull request makes Send live", async () => {
  await opened(reviewAtDelivery());
  const remarks = page.getByRole("region", { name: "Comments on the pull request" });
  await expect.element(remarks).toBeVisible();
  const picks = remarks.getByRole("checkbox").elements() as HTMLElement[];
  const send = remarks.getByRole("button", { name: "Send to a drone" });
  await expect.element(send).toBeDisabled();
  await userEvent.click(picks[0]!);
  await userEvent.click(picks[1]!);
  await expect.element(send).toBeEnabled();
});

// **The run tree and *Where things are* came off Overview on 29 Sep 2026.**
// Three claims stood here: the run reading while the Job's own read is out
// with the branch drawn from what the Board held, the workflow's diagram
// standing in for the tree before approval with each step's Checks showing,
// and the tree coming back once approved. The Workflow destination draws the
// run now — as a canvas, with a Check count per step rather than each Check's
// command — so none of the three has a subject on this screen.

test("the header's one control opens the rest of what this Job can do", async () => {
  await opened(recorded("done-worktree-given-back"));
  await page.getByRole("button", { name: "Everything else this job can do" }).click();
  await expect.element(page.getByRole("menuitem", { name: /record/i })).toBeVisible();
});

// **A Check failing on a test another Job is already fixing** — the lead
// names the fix, and so does that Check's row on the Record. The owner's
// decision of 2 Oct 2026, #1673; both claims stood here before #1671 took the
// region that drew them.
test("a Check failed on a test another Job is fixing: the row names the fix", async () => {
  await opened(withBreakages((jobId) => [brokenOnMain(FIX_JOB, jobId)], retryingCheckFailure()));
  await expect.element(page.getByRole("heading", { name: "cargo_nextest failed" })).toBeVisible();
  await expect.element(page.getByRole("button", { name: FIX_TITLE, exact: true })).toBeVisible();
  await expect.poll(text).toContain(`${FIX_TITLE} is already fixing this, and this Job is kept off the test's files`);
  await page.getByRole("tab", { name: "Record" }).click();
  await expect.element(page.getByText(`Failed — ${FIX_TITLE} is already fixing it`)).toBeVisible();
});

// **The Jobs themselves, and no count beside them** — the owner's note of
// 2 Oct 2026 on the count this claim first held: *"This should show the jobs
// that are waiting on this job."*
for (const [title, handle] of [
  ["Trim the brief to the files the step touched", "91-parked"],
  ["Memoise the manifest list", "92-parked-too"],
]) {
  test(`this Job is the fix, and two Jobs wait on it: each is listed, and ${handle} opens`, async () => {
    mount(scenarioNamed("breakage/fixed-elsewhere")!);
    await page.getByRole("button", { name: FIX_TITLE, exact: true }).click();
    const parked = page.getByRole("list", { name: "Waiting on this fix" });
    await expect.element(parked).toBeVisible();
    expect(parked.getByRole("button").elements()).toHaveLength(2);
    expect(text()).not.toMatch(/Jobs? waits? on it/);
    await parked.getByRole("button", { name: title, exact: true }).click();
    await expect.element(page.getByRole("button", { name: handle, exact: true })).toBeVisible();
  });
}

const text = () => document.body.textContent ?? "";

test("frozen, queued: the status says so, and names what it waits for", async () => {
  await opened(withRow(queued(), { queued_reason: "frozen", frozen_by: ["armada"] }));
  await expect.element(page.getByText("Frozen", { exact: true }).first()).toBeVisible();
  await expect.poll(text).toMatch(/Waits for\s*armada\s*to unfreeze/);
});

test("frozen, at review: the header says nothing lands until the repository unfreezes", async () => {
  await opened(withRow(reviewAtDelivery(), { frozen_by: ["armada"] }));
  await expect.poll(text).toMatch(/Nothing lands until\s*armada\s*unfreezes/);
});

test("Merge confirmed while frozen is taken, waiting, and never drawn as a refusal", async () => {
  await opened(withRow(reviewAtDelivery(), { frozen_by: ["armada"] }));
  await page.getByRole("button", { name: /^Merge(?! line)/ }).first().click();
  const confirm = page.getByRole("dialog");
  await entered(confirm);
  await confirm.getByRole("button", { name: "Merge pull request" }).click();
  await expect.element(page.getByText("Merge taken")).toBeVisible();
  await expect.element(page.getByText(/merges when the freeze lifts/)).toBeVisible();
});

// Fails today: App draws an open Job ahead of the composer, so `n` sets it open
// and nothing shows until the Job closes — #1242. Remove `.fails` with the fix.
test.fails("n opens the composer from a Job's detail", async () => {
  await opened(running());
  await userEvent.keyboard("n");
  await expect.element(page.getByText("Pick the repository this Job is for")).toBeVisible();
});

// **`n` into a drop reason** needed a drop reason to type into, and `Drop…`
// went with the Plan region. The claim underneath — that a key bound at the
// screen is typing while a field has focus — is the composer's, and the Plan
// destination is where it will be made again.

// **Selecting a step, and what the selection does when the Job advances**,
// was the step inspector's — `.armada-inside__step-name`, a panel beside the
// run tree. Both went with the Overview reframe, so the two claims here (the
// panel following the running step, and holding where a person put it) have
// nothing to follow or hold.

// **The gaming check's card, under Overview's lead and in the Workflow step
// panel** (owner, 2 Oct 2026, #1672). It had no renderer from the Overview
// reframe of 29 Sep 2026 until then, and a held Job sat until it was killed.
// One claim per shape Send it back takes, and one for where the block draws.

test("held by the gaming check with the Drone still there: Send it back redirects with the flag and the note", async () => {
  const api = await opened(heldByTheGamingCheck(["override_verdict", "redirect_drone", "redispatch_job"]));
  const overrule = vi.spyOn(api, "overrideVerdict");
  const redirect = vi.spyOn(api, "redirectDrone");
  const restart = vi.spyOn(api, "restartStep");
  const sendBack = page.getByRole("button", { name: "Send it back" });
  await expect.element(sendBack).toHaveAccessibleDescription(/Sends the flag back to the drone still on this step/);
  await userEvent.type(page.getByRole("textbox", { name: "Note (optional)" }), "Put it back");
  await sendBack.click();
  expect(redirect).toHaveBeenCalledWith(JOB_ID, expect.stringContaining("which was judged to weaken the test coverage."));
  expect(redirect).toHaveBeenCalledWith(JOB_ID, expect.stringContaining("The person's note: Put it back"));
  expect(restart).not.toHaveBeenCalled();
  await page.getByRole("button", { name: "Carry on" }).click();
  await expect.poll(() => overrule.mock.calls.length).toBe(1);
  expect(overrule).toHaveBeenCalledWith(JOB_ID, "");
});

test("held by the gaming check with the Drone gone: Send it back restarts the step", async () => {
  const api = await opened(heldByTheGamingCheck(["override_verdict", "restart_step", "redispatch_job"]));
  const redirect = vi.spyOn(api, "redirectDrone");
  const restart = vi.spyOn(api, "restartStep");
  const sendBack = page.getByRole("button", { name: "Send it back" });
  await expect.element(sendBack).toHaveAccessibleDescription(/Restarts the step with a fresh drone/);
  await sendBack.click();
  await expect.poll(() => restart.mock.calls.length).toBe(1);
  expect(restart).toHaveBeenCalledWith(JOB_ID, undefined);
  expect(redirect).not.toHaveBeenCalled();
});

test("held by the gaming check: the lead names the refused commands, and the step panel draws the same block", async () => {
  await opened(heldByTheGamingCheck(["override_verdict", "redirect_drone", "redispatch_job"]));
  await expect.element(page.getByText("3 commands were refused during Regression check")).toBeVisible();
  await expect.element(page.getByText(/^An assertion was removed or loosened, which was judged to weaken .* ·/)).toBeVisible();
  expect(page.getByText(/This Job stopped at/).query()).toBeNull();
  await page.getByRole("tab", { name: /^Workflow/ }).last().click();
  await page.getByRole("button", { name: /^Regression check, / }).last().click();
  const asks = page.getByRole("region", { name: "Question for you" }).last();
  await expect.element(asks.getByRole("group", { name: "Answer the flag" })).toBeVisible();
  // The flagged hunk, as Overview draws it, and not the citation alone: a
  // line of context only the patch carries.
  await expect.element(asks.getByText(/expect\(next\.version\)/)).toBeVisible();
  await expect.element(asks.getByRole("button", { name: "Send it back" })).toBeVisible();
  await expect.element(asks.getByRole("button", { name: "Carry on" })).toBeVisible();
});
