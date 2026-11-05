// The connectors switches in System › Repository, through `App` on the mock Fleet: a press saves the person's choice for
// that repository, and the switch reads it back. The walk `token-usage-settings` plays the same steps, and its last
// step claims ledger off, so this checks the claim the walk's engine cannot.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";
import { walkThrough } from "./walk";
import { WALKS } from "./walks";

unmountAfterEach();

const connectors = (name: string) =>
  page.getByRole("switch", {
    name: `Connectors in ${name}`,
  });

test("the walk ends on ledger's connectors switch pressed off, and the other repositories keep theirs", async () => {
  mount("token-usage-settings");
  await onScreen();
  const steps = WALKS.get("token-usage-settings")!.steps;
  // A press is seen only after Next performs it: the step that says ledger is off has to come after the press.
  expect(steps.at(-1)).toMatchObject({
    look: {
      role: "switch",
      name: "Connectors in ledger",
    },
  });
  await walkThrough(steps.slice(0, -2));
  await expect.element(connectors("ledger")).toBeChecked();

  await walkThrough(steps.slice(-2));
  await expect.element(connectors("ledger")).not.toBeChecked();
  await expect.element(connectors("armada")).not.toBeChecked();
  await expect.element(connectors("storefront")).toBeChecked();
});
