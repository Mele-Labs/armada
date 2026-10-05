// Where a criterion's words came from: Fleet's own word since 23.8 (#1642),
// and before it the Job's own origin (owner, 1 Oct 2026, `#1748` row 17).
// Every criterion read *From your prompt*, on a Job Fleet found itself as
// well. Where nothing names where the words came from, the slot stays empty.
//
// **A mark since 3 Oct 2026** (the owner: a state is never text), named on
// hover — so these find it by its role and name.

import { beforeEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import { refactorAtApproval, withRow } from "./job-detail-fixtures";
import { proposalFromAnIssue } from "./proposal-from-an-issue";
import { onJob } from "./scenario";
import { closeNode, mount, openNode, unmountAfterEach } from "./testing";

unmountAfterEach();

beforeEach(() => window.localStorage.setItem("armada.bridge.plan-lead-open", "true"));

// The approval canvas's Done when node, opened (prototype, 4 Oct 2026).
const doneWhen = () => page.getByRole("dialog", { name: "Done when" }).getByRole("region", { name: "Done when" });
const heldTo = () => page.getByRole("list").filter({ hasText: "Guide 8 is removed from the catalogue" }).last();
const FROM_REQUEST = { name: "From your request" };
const MOVED = { name: /^The issue has been edited since Fleet read it/ };

test("a Job Fleet found draws no origin on its criteria, on the approval panel or on Plan", async () => {
  mount(onJob(withRow(refactorAtApproval(), { origin: "auto_detected" })));
  await openNode("Done when");
  await expect
    .element(doneWhen().getByRole("textbox", { name: "Criterion 1" }))
    .toHaveValue("Guide 8 is removed from the catalogue");
  await expect.element(doneWhen().getByText("The Judge will decide it").first()).toBeVisible();
  expect(doneWhen().getByRole("img", FROM_REQUEST).elements()).toHaveLength(0);

  await closeNode();
  await page.getByRole("tab", { name: /^Plan/ }).last().click();
  await expect.element(heldTo()).toBeVisible();
  await expect.element(heldTo().getByText("The Judge will decide it", { exact: true }).first()).toBeVisible();
  expect(heldTo().getByRole("img", FROM_REQUEST).elements()).toHaveLength(0);
});

test("a Job a person dispatched still says its criteria came from their request", async () => {
  mount(onJob(withRow(refactorAtApproval(), { origin: "manual" })));
  await openNode("Done when");
  await expect.element(doneWhen().getByRole("img", FROM_REQUEST).first()).toBeVisible();

  await closeNode();
  await page.getByRole("tab", { name: /^Plan/ }).last().click();
  await expect.element(heldTo().getByRole("img", FROM_REQUEST).first()).toBeVisible();
});

test("a criterion Fleet read from an issue names the issue, and says the issue moved since", async () => {
  mount(onJob(proposalFromAnIssue()));
  await openNode("Done when");
  await expect.element(doneWhen().getByText("armada#1162").first()).toBeVisible();
  // Two lines are the issue's and one the prompt's, each as Fleet said.
  expect(doneWhen().getByText("armada#1162").elements()).toHaveLength(2);
  expect(doneWhen().getByRole("img", FROM_REQUEST).elements()).toHaveLength(1);
  expect(doneWhen().getByRole("img", MOVED).elements()).toHaveLength(2);
  // Since Fleet read it, never since the words froze: nothing is frozen at the gate.
  expect(doneWhen().getByText(/since these words were frozen/).elements()).toHaveLength(0);

  await closeNode();
  await page.getByRole("tab", { name: /^Plan/ }).last().click();
  await expect.element(heldTo().getByText("armada#1162").first()).toBeVisible();
  expect(heldTo().getByRole("img", MOVED).elements()).toHaveLength(2);
});
