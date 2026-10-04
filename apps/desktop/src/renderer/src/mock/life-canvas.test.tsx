// The approval canvas past the gate (prototype, 4 Oct 2026): every card reads,
// and nothing on it can change any more.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, openNode, unmountAfterEach } from "./testing";

unmountAfterEach();

test("past the gate a step opens Workflow's own panel, and a card with no panel elsewhere reads", async () => {
  mount("proto/feature-running");
  await expect.element(page.getByRole("region", { name: "This Job's run" })).toBeVisible();
  // It opens on where the Job is, at full size; Fit brings the whole run in.
  await page.getByRole("button", { name: "Fit" }).click();
  await page.getByRole("button", { name: /^Plan the change, / }).click();
  await expect.element(page.getByRole("tab", { name: /^Workflow/, selected: true })).toBeVisible();
  await page.getByRole("button", { name: "Back to Overview" }).click();
  await expect.element(page.getByRole("region", { name: "This Job's run" })).toBeVisible();
  await page.getByRole("button", { name: "Fit" }).click();
  const brief = await openNode("Brief");
  await expect.element(brief.getByRole("region", { name: "Done when" })).toBeVisible();
  expect(brief.getByRole("textbox").all()).toHaveLength(0);
  expect(brief.getByRole("button", { name: "Send to proposer" }).all()).toHaveLength(0);
});

test("an Epic Job's run draws its wave once, on the canvas", async () => {
  mount("proto/epic-running");
  await expect.element(page.getByRole("region", { name: "This Job's run" })).toBeVisible();
  await expect.element(page.getByRole("button", { name: /^Drop the second error shape, / })).toBeVisible();
  expect(page.getByRole("region", { name: "The wave", exact: true }).all()).toHaveLength(0);
});
