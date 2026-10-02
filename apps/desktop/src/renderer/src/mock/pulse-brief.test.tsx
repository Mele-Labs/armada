// A kept brief, read in Pulse's log panel, through `App`.
//
// The owner, 2 Oct 2026: a Judge's or a gaming check's brief opens in the log
// panel like a transcript does, off what Fleet serves (`get_brief`, 21.11),
// and the row's own Open still leaves the app.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { escalatedEvidenceSuspect } from "@armada/screens/src/fixtures/build/index";

import { onJob } from "./scenario";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

async function onPulse(): Promise<void> {
  await page.getByRole("tab", { name: "Pulse" }).click();
  await expect.element(page.getByRole("tabpanel", { name: "Pulse" })).toBeVisible();
}

test("pressing a Judge brief's row reads the brief in the panel, ending on its question", async () => {
  mount(onJob(escalatedEvidenceSuspect()));
  await onPulse();

  await page.getByRole("button", { name: "Judge brief, regression_verify · c2" }).click();

  const panel = page.getByRole("dialog", { name: "Judge brief" });
  await expect.element(panel).toBeVisible();
  await expect.element(panel.getByText("regression_verify · c2")).toBeVisible();
  await expect
    .element(panel.getByText(/still assert that selectVisibleColumns memoises/))
    .toBeVisible();
  // Numbered as the file is, so a Judge's citation lands on the line it names.
  await expect.element(panel.getByText(/regression_verify\.1\.c2\.md/)).toBeVisible();
});

test("a gaming check's brief opens the same way, and its row keeps the Open that leaves the app", async () => {
  mount(onJob(escalatedEvidenceSuspect()));
  await onPulse();

  const row = page.getByRole("listitem").filter({ hasText: "regression_verify · gaming check" });
  await expect.element(row.getByRole("button", { name: "Open" })).toBeVisible();

  await page.getByRole("button", { name: "Judge brief, regression_verify · gaming check" }).click();

  const panel = page.getByRole("dialog", { name: "Judge brief" });
  await expect.element(panel.getByText(/asserts less than it did/)).toBeVisible();
});

test("a brief Fleet no longer holds says so inside the panel", async () => {
  mount(onJob({ ...escalatedEvidenceSuspect(), briefs: {} }));
  await onPulse();

  await page.getByRole("button", { name: "Judge brief, regression_verify · c1" }).click();

  const panel = page.getByRole("dialog", { name: "Judge brief" });
  await expect.element(panel.getByText(/no longer in the record|did not answer/)).toBeVisible();
});
