// The + between steps opens its menu over the canvas, never behind the next node (the owner,
// 7 Oct 2026: "Write tests" covered it). The menu is drawn in a layer of its own on the page, so what
// is under the middle of each item is the item. Asked at every place a menu opens.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

/** Whether the element at the middle of each menu item is that item or inside it. */
function itemsAreOnTop(): boolean {
  const items = [...document.querySelectorAll<HTMLElement>('[role="menu"] [role="menuitem"]')];
  return (
    items.length === 3 &&
    items.every((item) => {
      const box = item.getBoundingClientRect();
      const hit = document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
      return hit !== null && item.contains(hit);
    })
  );
}

async function pressed(name: string): Promise<void> {
  await page.getByRole("button", { name }).click();
  await expect.element(page.getByRole("menuitem", { name: "Script" })).toBeVisible();
  expect(itemsAreOnTop()).toBe(true);
}

test("the menu on a running Job's canvas is over the next step", async () => {
  mount("proto/feature-running");
  await onScreen();
  await page.getByRole("tab", { name: "Workflow" }).click();
  await pressed("Add a step after Implement");
});

// Every gap at the gate: ahead of the first step, between steps, the end of the Work lane, and the
// Delivery lane before the pull request opens, after it opens, and after the step that delivers.
for (const name of [
  "Add a step before Plan the change",
  "Add a step after Plan the change",
  "Add a step after Implement",
  "Add a step after Done when",
  "Add a step after Pull request",
  "Add a step after Review the change",
]) {
  test(`the menu at the gate is over the next node: ${name}`, async () => {
    mount("proto/feature-at-approval");
    await onScreen();
    await pressed(name);
  });
}
