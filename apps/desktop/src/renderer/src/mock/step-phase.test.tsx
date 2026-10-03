// Which part of a running step it is in, on the Workflow card and in the step
// panel's header — a mark named by its tooltip, never a phrase (owner's
// annotation of 3 Oct 2026, `ouqa`: "I had no idea it was running checks until
// i finally noticed that the checks were updating in the panel").

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

async function workflowOf(scenario: string): Promise<void> {
  mount(scenario);
  await onScreen();
  await page.getByRole("tab", { name: /^Workflow/ }).last().click();
}

test("check-logs: the running card and its panel say the Checks are running", async () => {
  await workflowOf("check-logs");
  const card = page.getByRole("button", { name: "Implement, running, Checks running" }).last();
  await expect.element(card).toBeVisible();
  await card.click();
  const panel = page.getByRole("dialog", { name: "Implement" }).last();
  await expect.element(panel.getByRole("img", { name: "Checks running" })).toBeVisible();
});

test("arc/executing-sequential: the running card names the Drone's task", async () => {
  await workflowOf("arc/executing-sequential");
  await expect
    .element(page.getByRole("button", { name: /^Implement, running, Drone on T\d+$/ }).last())
    .toBeVisible();
});
