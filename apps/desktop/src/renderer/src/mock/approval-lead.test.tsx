// A Job waiting for its dispatch to be approved, through `App`, **on a Fleet
// that serves no draft** — which is every real one until #1545. The owner
// dispatched one from the composer on 1 Oct 2026 and Overview's lead said
// *Waiting for your approval* and nothing else: its act reached the proposal,
// and with no proposal read the screen offered no act at all. *"The action is
// in the upper right corner of the app. It should be in both places."*

import { expect, test, vi } from "vitest";
import { page } from "vitest/browser";
import { awaitingApproval } from "@armada/screens/src/fixtures/build/index";
import { JOB_ID } from "@armada/screens/src/fixtures/build/base";

import { withRow } from "./job-detail-fixtures";
import { onJob } from "./scenario";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

/**
 * The owner's Job 1 as Fleet served it: no branch, no Drone, no draft beside
 * it. The fixture's own steps stand in for `refactor`'s, which the lead does
 * not read.
 */
const REAL = () =>
  withRow(awaitingApproval(), {
    handle: "1-retire-guide-8-and-add-guide-validation-ru",
    title: "Retire guide 8 and add guide validation rule",
    origin: "auto_detected",
    model: "sonnet",
  });

/** App, on that Job's Overview, and the fake it talks to. */
async function opened() {
  const app = mount(onJob(REAL()));
  await expect.element(page.getByRole("heading", { name: "Waiting for your approval" })).toBeVisible();
  return app.api;
}

/** One region of the screen, as a locator a press can be scoped to. */
function within(selector: string) {
  const found = document.querySelector<HTMLElement>(selector);
  if (found === null) throw new Error(`nothing matched ${selector}`);
  return page.elementLocator(found);
}

const lead = () => within(".armada-lead");
const header = () => within(".armada-job-head");

test("the lead holds Approve dispatch beside the header's, with the same menu", async () => {
  await opened();
  await expect.element(lead().getByRole("button", { name: "Approve dispatch" })).toBeVisible();
  await expect.element(header().getByRole("button", { name: "Approve dispatch" })).toBeVisible();
  // One act reached two ways, so the caret offers what the header's does.
  await lead().getByRole("button", { name: "Everything else this job can do" }).click();
  await expect.element(page.getByRole("menuitem", { name: /Kill job/ })).toBeVisible();
});

test("pressing the lead's Approve dispatch approves, as the header's does", async () => {
  const api = await opened();
  const approveDispatch = vi.spyOn(api, "approveDispatch");
  await lead().getByRole("button", { name: "Approve dispatch" }).click();
  expect(approveDispatch).toHaveBeenCalledWith(JOB_ID);
});

test("pressing the header's Approve dispatch sends the same thing", async () => {
  const api = await opened();
  const approveDispatch = vi.spyOn(api, "approveDispatch");
  await header().getByRole("button", { name: "Approve dispatch" }).click();
  expect(approveDispatch).toHaveBeenCalledWith(JOB_ID);
});
