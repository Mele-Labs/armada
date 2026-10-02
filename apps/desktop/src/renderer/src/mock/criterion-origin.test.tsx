// Where a criterion's words came from follows the Job's own origin (owner,
// 1 Oct 2026, `#1748` row 17). Every criterion read *From your prompt*, on a
// Job Fleet found itself as well. Where no person dispatched the Job nothing
// names where the words came from, and the slot stays empty.

import { beforeEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import { refactorAtApproval, withRow } from "./job-detail-fixtures";
import { onJob } from "./scenario";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

beforeEach(() => window.localStorage.setItem("armada.bridge.plan-lead-open", "true"));

const doneWhen = () => page.getByRole("region", { name: "What you are approving" }).getByRole("region", { name: "Done when" });
const heldTo = () => page.getByRole("list").filter({ hasText: "Guide 8 is removed from the catalogue" }).last();

test("a Job Fleet found draws no origin on its criteria, on the approval panel or on Plan", async () => {
  mount(onJob(withRow(refactorAtApproval(), { origin: "auto_detected" })));
  await expect.element(doneWhen().getByText("Guide 8 is removed from the catalogue")).toBeVisible();
  expect(doneWhen().getByText(/From your prompt/).elements()).toHaveLength(0);
  await expect.element(doneWhen().getByText("The Judge will decide it").first()).toBeVisible();

  await page.getByRole("tab", { name: /^Plan/ }).last().click();
  await expect.element(heldTo()).toBeVisible();
  expect(heldTo().getByText(/From your prompt/).elements()).toHaveLength(0);
  await expect.element(heldTo().getByText("The Judge will decide it", { exact: true }).first()).toBeVisible();
});

test("a Job a person dispatched still says its criteria came from their prompt", async () => {
  mount(onJob(withRow(refactorAtApproval(), { origin: "manual" })));
  await expect.element(doneWhen().getByText(/From your prompt/).first()).toBeVisible();

  await page.getByRole("tab", { name: /^Plan/ }).last().click();
  await expect.element(heldTo().getByText("From your prompt · The Judge will decide it").first()).toBeVisible();
});
