// The Job Board absorbed into Overview — the owner, 28 Sep 2026: *"Overview now
// supersedes the job board. We should delete this page."*
//
// **What Overview had to gain first.** The Board carried readings and acts that
// existed nowhere else, so the page could not simply go: the Done section, and
// the menu holding Reported, Refresh and the two bulk sweeps. Each is pinned
// here, because each is a thing a person could do before and must still be able
// to do.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

/** Overview's own menu, which is the whole of the control since 28 Sep. */
async function everythingElse(): Promise<void> {
  await page.getByRole("button", { name: "Everything else" }).click();
}

/**
 * The owner, 28 Sep 2026: *"We dont need this button on the overview because
 * its already right above it in the title bar."* The menu's face and the title
 * row's button called one `onCompose`, so the face went and the menu stayed.
 */
test("Overview carries the menu and no Dispatch of its own, since the title row has one", async () => {
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
  await expect.element(page.getByRole("button", { name: "Everything else" })).toBeVisible();
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

test("Done draws every Job that completed or was cleared, folded until asked for", async () => {
  mount("every-state");
  await onScreen();

  // Folded: the heading is there and the rows are not, which is the whole
  // point of what is over being read on purpose.
  // Folded, so the head says Expand rather than Collapse — every other
  // section opens by default and this one does not.
  const done = page.getByRole("button", { name: "Expand Done" });
  await expect.element(done).toBeVisible();

  await done.click();
  // A Job the Board's Done tab held and Overview did not draw at all before.
  await expect.element(page.getByRole("button", { name: "Collapse Done" })).toBeVisible();
  expect(document.querySelectorAll("#armada-overview-panel-done [data-job-id]").length).toBeGreaterThan(0);
});

test("Reported is reachable, and it was only ever in the Board's menu", async () => {
  mount("every-state");
  await onScreen();

  await everythingElse();
  await page.getByRole("menuitem", { name: "Reported" }).click();
  // The reports screen, by its own way out — the mock serves no `/reports`, so
  // what it draws is the refusal and this control.
  await expect.element(page.getByRole("button", { name: "Back to the list" })).toBeVisible();
});

test("Refresh and both bulk sweeps are on the menu Overview now carries", async () => {
  mount("every-state");
  await onScreen();

  await everythingElse();
  await expect.element(page.getByRole("menuitem", { name: "Refresh" })).toBeVisible();
  await expect.element(page.getByRole("menuitem", { name: /^Clear \d+ finished/ })).toBeVisible();
  await expect.element(page.getByRole("menuitem", { name: /records$/ })).toBeVisible();
});

test("a bulk clear asks before it sweeps, from Overview as it did from the Board", async () => {
  mount("every-state");
  await onScreen();

  await everythingElse();
  await page.getByRole("menuitem", { name: /^Clear \d+ finished/ }).click();
  const asking = page.getByRole("dialog");
  await expect.element(asking).toBeVisible();
  await expect.element(asking.getByRole("button", { name: "Clear", exact: true })).toBeVisible();
});
