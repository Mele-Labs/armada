// A step Fleet refuses to add is said in Fleet's own sentence where every command's refusal is said,
// and the panel it was filled in on stays open with what was typed, so nothing is lost.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const SAID = "step `implement` has already started, so a step cannot go before it";

test("a refused add is said as Fleet said it, and the panel stays", async () => {
  const app = mount("proto/feature-running");
  app.api.addJobStep = async () => ({
    ok: false,
    outcome: { ok: false, why: "refused", error: { code: "fleet.added_step_behind", message: SAID, run_id: "mock", fields: {}, chain: [] } },
  });
  await onScreen();
  await page.getByRole("tab", { name: "Workflow" }).click();
  await page.getByRole("button", { name: "Add a step after Implement" }).click();
  await page.getByRole("menuitem", { name: "Skill" }).click();
  await page.getByRole("textbox", { name: "Skill" }).fill("qa-notes");
  await page.getByRole("button", { name: "Add", exact: true }).click();
  await expect.element(page.getByText(SAID).first()).toBeVisible();
  await expect.element(page.getByRole("textbox", { name: "Skill" })).toHaveValue("qa-notes");
});
