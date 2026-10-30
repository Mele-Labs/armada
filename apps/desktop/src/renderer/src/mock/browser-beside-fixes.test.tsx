// Three layout fixes, read off the real layout in the browser: the Jobs list's filter chip lies clear of the first row, the Job's
// thread sits close to its browser, and Send stays inside the thread's composer beside the browser. Over the `job-browser` scenario.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, toPlace, unmountAfterEach } from "./testing";

unmountAfterEach();

const box = (element: Element) => element.getBoundingClientRect();

test("the Jobs list's filter chip does not lie over the first row", async () => {
  await page.viewport(1440, 900);
  mount("job-browser");
  await onScreen();
  await toPlace("Cockpit");
  const list = page.getByRole("region", { name: "Jobs", exact: true });
  await list.hover();
  const chip = list.element().querySelector(".deck-band")!;
  const row = list.element().querySelector(".deck-row")!;
  await expect.poll(() => getComputedStyle(chip).opacity).toBe("1");
  expect(box(chip).bottom).toBeLessThanOrEqual(box(row).top);
});

/** The Job open beside its browser, at the width the thread is first given. */
async function besideTheBrowser(): Promise<{ beside: Element; sheet: Element }> {
  await page.viewport(1440, 900);
  mount("job-browser");
  await onScreen();
  await toPlace("Cockpit");
  await page.getByRole("button", { name: /Fix the checkout total/ }).first().click();
  await page.getByRole("region", { name: "Flight controls" }).getByRole("button", { name: "Browser", exact: true }).click();
  const beside = document.querySelector(".job-browser-beside")!;
  await expect.poll(() => box(beside).width).toBeGreaterThan(0);
  return { beside, sheet: document.querySelector(".armada-sheet")! };
}

test("beside the browser the thread keeps one narrow gap to it", async () => {
  const { beside, sheet } = await besideTheBrowser();
  // The handle's `--space-2`, with nothing spent on a margin either side of it.
  expect(box(sheet).left - box(beside).right).toBeLessThanOrEqual(12);
});

test("beside the browser Send stays inside the thread's composer at the thread's narrowest", async () => {
  const { beside } = await besideTheBrowser();
  (beside as HTMLElement).style.width = "320px";
  await expect.poll(() => box(beside).width).toBe(320);
  const send = beside.querySelector<HTMLElement>('button[type="submit"]')!;
  expect(box(send).right).toBeLessThanOrEqual(box(beside).right);
  expect(box(send).left).toBeGreaterThanOrEqual(box(beside).left);
});
