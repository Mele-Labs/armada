// The title row's name is the repository picker, with the Manifest's chip beside it, on every surface the Cockpit included.
// The walk `title-bar-repository` draws it; this holds that the presses reach the same code.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const picker = () => page.getByRole("button", { name: "Repository", exact: true });

test("The picker and the Manifest chip are in the title row on the Cockpit and the Logbook, and a pick follows", async () => {
  mount("sessions-by-repository");
  await onScreen();
  await expect.element(picker()).toHaveTextContent("All repositories");
  expect(picker().element().closest(".armada-title-bar")).not.toBeNull();
  await expect.element(page.getByRole("button", { name: "Open the Manifest", exact: true })).toBeVisible();

  await userEvent.click(picker());
  await userEvent.click(page.getByRole("menuitem", { name: "storefront" }));
  await expect.element(picker()).toHaveTextContent("storefront");

  await userEvent.click(page.getByRole("tab", { name: "Logbook", exact: true }));
  await expect.element(picker()).toHaveTextContent("storefront");
  await expect.element(page.getByRole("button", { name: "Open the Manifest", exact: true })).toBeVisible();
});
