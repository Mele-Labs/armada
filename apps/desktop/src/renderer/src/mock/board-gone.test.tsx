// The Job Board absorbed into Overview — the owner, 28 Sep 2026: *"Overview now
// supersedes the job board. We should delete this page."*
//
// **What Overview had to gain first.** The Board carried readings and acts that
// existed nowhere else, so the page could not simply go: the Done section, and
// the menu holding Reported, Refresh and the two bulk sweeps. The menu went on
// 1 Oct 2026 and `overview-menu-gone.test.tsx` pins where each item lives now.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

/**
 * The owner, 28 Sep 2026: *"We dont need this button on the overview because
 * its already right above it in the title bar."* On 10 Oct 2026 the title
 * row's button went too: its one bar searches and dispatches. `one-bar.tsx`.
 */
test("Overview carries no Dispatch of its own, and the title row's one bar is the way in", async () => {
  mount("default");
  await onScreen();

  await expect.element(page.getByRole("button", { name: /^Search, or describe work to dispatch/ })).toBeVisible();
  expect(page.getByRole("button", { name: "Dispatch", exact: true }).query()).toBeNull();
});

test("the Job Board is gone from the title bar", async () => {
  mount("default");
  await onScreen();

  await expect.element(page.getByRole("navigation", { name: "Places" })).toBeVisible();
  expect(page.getByRole("tab", { name: "Job Board" }).query()).toBeNull();
  expect(page.getByRole("button", { name: "Job Board" }).query()).toBeNull();
});

test("the Board's old shortcut reaches the Cockpit, which holds the work a digit once named", async () => {
  mount("default");
  await onScreen();

  // `⌘2` was the Board's, then Studios', then Jobs'. The Cockpit holds Sessions alone now.
  await userEvent.keyboard("{Meta>}2{/Meta}");
  await expect.poll(() => document.querySelector('nav[aria-label="Places"] [role="tab"][aria-selected="true"]')?.textContent).toContain("Cockpit");
});
