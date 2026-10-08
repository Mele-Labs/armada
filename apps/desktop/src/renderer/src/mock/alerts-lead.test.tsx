// The mock Fleet's `list_alerts`, drawn in the lead of a Job's Overview: the held Job is a row naming it
// and its Trigger, a press opens it on the Workflow tab, and the row goes when the hold lets go.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

test("mock alerts: a Trigger that holds the Job is a row in the lead, and Rerun letting the hold go takes it away", async () => {
  mount("real/job-2-trigger-hold");
  await onScreen();
  const list = page.getByRole("list", { name: "Alerts" });
  await expect.element(list).toBeVisible();
  await expect.element(list.getByRole("button", { name: /deploy_qa/ })).toBeVisible();
  await page.getByRole("button", { name: "Rerun" }).first().click();
  await expect.element(list).not.toBeInTheDocument();
});

test("mock alerts: pressing the row opens the Job on its Workflow tab", async () => {
  mount("real/job-2-trigger-hold");
  await onScreen();
  await page.getByRole("list", { name: "Alerts" }).getByRole("button", { name: /deploy_qa/ }).click();
  await expect.element(page.getByRole("tab", { name: "Workflow" })).toHaveAttribute("aria-selected", "true");
});

test("mock alerts: a Job with no Trigger alert has no list", async () => {
  mount("real/job-2-trigger-repair");
  await onScreen();
  expect(page.getByRole("list", { name: "Alerts" }).elements()).toHaveLength(0);
});
