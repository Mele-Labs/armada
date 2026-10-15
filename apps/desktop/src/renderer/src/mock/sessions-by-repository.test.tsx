// The Sessions list through `App`: on All with two repositories served each is a band with the state
// columns inside, and one repository picked draws the list with no band.
// The walk `sessions-by-repository` draws it; this holds that the presses reach the same code.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

test("All draws a band for each repository, and one repository picked draws none", async () => {
  mount("sessions-by-repository");
  await onScreen();
  await userEvent.click(page.getByRole("button", { name: "Sessions", exact: true }));
  const list = page.getByRole("region", { name: "Sessions" });
  const armada = list.getByRole("region", { name: "armada" });
  const storefront = list.getByRole("region", { name: "storefront" });
  await expect.element(armada.getByRole("heading", { name: "Needs you" })).toBeVisible();
  await expect.element(armada.getByRole("button", { name: /Prune the stale branches/ })).toBeVisible();
  await expect.element(armada.getByRole("button", { name: /Round the cart totals/ })).not.toBeInTheDocument();
  await expect.element(storefront.getByRole("button", { name: /Round the cart totals/ })).toBeVisible();

  await userEvent.click(page.getByRole("button", { name: "All repositories" }));
  await userEvent.click(page.getByRole("menuitem", { name: "storefront" }));
  await expect.element(list.getByRole("button", { name: /Round the cart totals/ })).toBeVisible();
  await expect.element(list.getByRole("region", { name: "storefront" })).not.toBeInTheDocument();
  await expect.element(list.getByRole("region", { name: "armada" })).not.toBeInTheDocument();
});
