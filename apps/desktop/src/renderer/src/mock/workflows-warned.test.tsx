// The System rail marks Workflows only while Fleet says a file was left out, and draws no count.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, toPlace, unmountAfterEach } from "./testing";
import { workflowing } from "./workflows-fleet";

unmountAfterEach();

test("the sidebar warns only while Fleet says a file was left out, and draws no count", async () => {
  const scenario = workflowing();
  mount(scenario);
  await onScreen();
  await toPlace("System");
  await expect.element(page.getByRole("img", { name: "A workflow file cannot run" })).toBeVisible();
});

test("a Fleet that does not say a file was left out draws no mark", async () => {
  const scenario = workflowing();
  mount({
    ...scenario,
    state: { ...scenario.state, health: { state: "read", health: { probes: [], not_probed: [], helm_action_authority: "acting" } } },
  });
  await onScreen();
  await toPlace("System");
  await expect.element(page.getByRole("navigation", { name: "System" }).getByRole("button", { name: /^Workflows( |$)/ })).toBeVisible();
  await expect.element(page.getByRole("img", { name: "A workflow file cannot run" })).not.toBeInTheDocument();
});
