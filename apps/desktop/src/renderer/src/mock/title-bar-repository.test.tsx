// The title row names the picked repository on every surface but the Cockpit, which has the picker.
// The walk `title-bar-repository` draws it; this holds that the presses reach the same code.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

test("Retros names the pick in the title row, All says so, and the Cockpit draws no tag", async () => {
  mount("sessions-by-repository");
  await onScreen();
  const tag = () => document.querySelector(".armada-title-bar__repository");
  expect(tag()).toBeNull();

  await userEvent.click(page.getByRole("tab", { name: "Retros", exact: true }));
  await expect.element(page.getByTitle("All repositories")).toHaveTextContent("All repositories");

  // The pick is made on the Cockpit, where the picker is, and the title row follows it elsewhere.
  await userEvent.click(page.getByRole("tab", { name: "Cockpit" }));
  expect(tag()).toBeNull();
  await userEvent.click(page.getByText("All repositories"));
  await userEvent.click(page.getByRole("menuitem", { name: "storefront" }));
  expect(tag()).toBeNull();
  await userEvent.click(page.getByRole("tab", { name: "Retros", exact: true }));
  await expect.element(page.getByTitle("storefront")).toHaveTextContent("storefront");
});
