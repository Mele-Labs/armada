// Where a criterion's words came from: Fleet's own word since 23.8 (#1642),
// and before it the Job's own origin (owner, 1 Oct 2026, `#1748` row 17).
// Every criterion read *From your prompt*, on a Job Fleet found itself as
// well. Where nothing names where the words came from, the slot stays empty.

import { beforeEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import { refactorAtApproval, withRow } from "./job-detail-fixtures";
import { proposalFromAnIssue } from "./proposal-from-an-issue";
import { onJob } from "./scenario";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

beforeEach(() => window.localStorage.setItem("armada.bridge.plan-lead-open", "true"));

const doneWhen = () => page.getByRole("region", { name: "What you are approving" }).getByRole("region", { name: "Done when" });
const heldTo = () => page.getByRole("list").filter({ hasText: "Guide 8 is removed from the catalogue" }).last();

test("a Job Fleet found draws no origin on its criteria, on the approval panel or on Plan", async () => {
  mount(onJob(withRow(refactorAtApproval(), { origin: "auto_detected" })));
  await expect
    .element(doneWhen().getByRole("textbox", { name: "Criterion 1" }))
    .toHaveValue("Guide 8 is removed from the catalogue");
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

test("a criterion Fleet read from an issue names the issue, and says the issue moved since", async () => {
  mount(onJob(proposalFromAnIssue()));
  await expect.element(doneWhen().getByText("armada#1162").first()).toBeVisible();
  // Two lines are the issue's and one the prompt's, each as Fleet said.
  expect(doneWhen().getByText("armada#1162").elements()).toHaveLength(2);
  expect(doneWhen().getByText(/From your prompt/).elements()).toHaveLength(1);
  expect(doneWhen().getByRole("note").elements()).toHaveLength(2);

  await page.getByRole("tab", { name: /^Plan/ }).last().click();
  await expect.element(heldTo().getByText("armada#1162").first()).toBeVisible();
});
