// A press on a Job of a repository other than the picked one (a macOS banner, a note sent) switches the picked repository to that Job's
// and opens the Job on the Cockpit, rather than doing nothing. The scenario `cockpit-list-by-repository` serves two repositories.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { askToOpenSession } from "../open-session";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const STOREFRONT_JOB = "01STOREASKINGDDDDDDDDDDDDD";

test("opening a Job of another repository picks that repository and opens the Job", async () => {
  mount("cockpit-list-by-repository");
  await onScreen();
  // The window is on one repository, which leaves out the Job asked for.
  await userEvent.click(page.getByRole("button", { name: "Repository", exact: true }));
  await userEvent.click(page.getByRole("menuitem", { name: "armada Armada configuration" }));
  await expect.element(page.getByRole("button", { name: "Round the cart totals" })).not.toBeInTheDocument();

  askToOpenSession(STOREFRONT_JOB);

  await expect.element(page.getByRole("button", { name: "Repository", exact: true })).toHaveTextContent("storefront");
  await expect.element(page.getByRole("region", { name: "Job: Round the cart totals" })).toBeVisible();
});
