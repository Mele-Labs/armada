// The owner, 7 Oct 2026, on the Checks page: "Why are checks not showing up here?" The run sheet
// had not been read, and the page threw away the Checks it could have drawn without it.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, toPageByName, unmountAfterEach } from "./testing";

unmountAfterEach();

test("the merge line's Checks list while the run sheet is unread", async () => {
  mount("checks-without-run-sheet");
  await onScreen();

  await toPageByName("Checks");
  await expect.element(page.getByRole("button", { name: "desktop_test", exact: true })).toBeVisible();
  // The checkout's own Check is the sheet's, and is not drawn.
  expect(page.getByRole("button", { name: "test", exact: true }).query()).toBeNull();
});
