// The owner, 7 Oct 2026, on the Checks page: "Why are checks not showing up here?" The run sheet
// had not been read, and the page threw away every Check a Job or a Drone had reported with it.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

test("a gate's, a Drone's and the merge line's Checks list while the run sheet is unread", async () => {
  mount("checks-without-run-sheet");
  await onScreen();

  await page.getByRole("navigation", { name: "Work" }).getByRole("button", { name: "Checks", exact: true }).click();
  await expect.element(page.getByRole("button", { name: "components_test", exact: true })).toBeVisible();
  await expect.element(page.getByRole("button", { name: "scripts_test", exact: true })).toBeVisible();
  await expect.element(page.getByRole("button", { name: "desktop_test", exact: true })).toBeVisible();
  // The checkout's own Check is the sheet's, and is not drawn.
  expect(page.getByRole("button", { name: "test", exact: true }).query()).toBeNull();
});
