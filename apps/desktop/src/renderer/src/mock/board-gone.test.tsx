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
 * its already right above it in the title bar."* The menu's face and the title
 * row's button called one `onCompose`, so the face went and the menu stayed.
 */
test("Overview carries no Dispatch of its own, since the title row has one", async () => {
  mount("every-state");
  await onScreen();

  const dispatch = page.getByRole("button", { name: "Dispatch", exact: true });
  await expect.element(dispatch).toBeVisible();
  // One, and it is the title row's: a second on the surface below it is the
  // duplicate that was cut.
  expect(dispatch.all()).toHaveLength(1);
  expect(
    page.getByRole("region", { name: "Overview" }).getByRole("button", { name: "Dispatch", exact: true }).query(),
  ).toBeNull();
});

test("the Job Board is gone from the rail", async () => {
  mount("every-state");
  await onScreen();

  await expect.element(page.getByRole("navigation", { name: "Work" })).toBeVisible();
  expect(page.getByRole("button", { name: "Job Board" }).query()).toBeNull();
});

test("the Board's old shortcut reaches Studios, because the digits closed up behind it", async () => {
  mount("every-state");
  await onScreen();

  // `⌘2` was the Board's. A digit left reaching nothing is what "no gap"
  // forbids; Studios moved up into it.
  await userEvent.keyboard("{Meta>}2{/Meta}");
  await expect.poll(() =>
    document.querySelector('nav[aria-label="Work"] [aria-current="page"]')?.textContent,
  ).toContain("Studios");
});

test("Done draws every Job that completed or was cleared, on its own tab", async () => {
  mount("every-state");
  await onScreen();

  // What is over is read on purpose: its own tab, and not the one Overview opens on.
  const done = page.getByRole("tab", { name: "Done" });
  await expect.element(done).toHaveAttribute("aria-selected", "false");
  await done.click();
  // **Polled, not read.** The tab flipping is one render and its rows are the next.
  await expect.poll(() => document.querySelectorAll('[role="listbox"][aria-label="Items"] [data-job-id]').length).toBeGreaterThan(0);
});
