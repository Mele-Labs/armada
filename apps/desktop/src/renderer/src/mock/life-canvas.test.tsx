// The approval canvas past the gate (prototype, 4 Oct 2026): every card reads,
// and nothing on it can change any more.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, openNode, unmountAfterEach } from "./testing";

unmountAfterEach();

test("past the gate every card reads, and none offers a control", async () => {
  mount("proto/feature-running");
  await expect.element(page.getByRole("region", { name: "This Job's run" })).toBeVisible();
  const plan = await openNode("Plan the change");
  await expect.element(plan.getByText("Auto").first()).toBeVisible();
  expect(plan.getByRole("combobox").all()).toHaveLength(0);
  // It opens on where the Job is, so Brief is panned past; Fit brings it back.
  await page.getByRole("button", { name: "Close" }).last().click();
  await page.getByRole("button", { name: "Fit" }).click();
  const brief = await openNode("Brief");
  await expect.element(brief.getByRole("region", { name: "Done when" })).toBeVisible();
  expect(brief.getByRole("textbox").all()).toHaveLength(0);
  expect(brief.getByRole("button", { name: "Send to proposer" }).all()).toHaveLength(0);
});
