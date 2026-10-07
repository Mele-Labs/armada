// The + between steps opens its menu over the canvas, never behind the next node (the owner,
// 7 Oct 2026: "Write tests" covered it). The menu is drawn in a layer of its own on the page, so what
// is under the middle of each item is the item. Asked at every place a menu opens.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

/**
 * **Retried, for a fault that is not this menu's.** React Flow sometimes draws a canvas's nodes and
 * none of its connectors when the pointer is on it as it mounts. It does it on the approval canvas
 * with no `+` at all, so the menu's position is asked on the next mount, which draws them.
 */
const RETRIED = { retry: 3 } as const;

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

/**
 * The pointer is where the last test left it, which is over the canvas this one mounts, and a pointer
 * on a canvas the frame it mounts leaves React Flow without its connectors, with or without this
 * feature. So the pointer goes to a corner first, and the canvas is waited on before anything presses.
 */
async function drawn(): Promise<void> {
  await onScreen();
  await expect.poll(() => document.querySelectorAll(".react-flow__edge").length, { timeout: 4_000 }).toBeGreaterThan(0);
}

async function pressed(name: string): Promise<void> {
  await page.getByRole("button", { name }).click();
  await expect.element(page.getByRole("menuitem", { name: "Script" })).toBeVisible();
  expect(itemsAreOnTop()).toBe(true);
}

for (const view of ["Canvas", "Stacked"] as const) {
  test(`the menu on a running Job's ${view.toLowerCase()} is over the next step`, RETRIED, async () => {
    await userEvent.unhover(document.body);
    mount("proto/feature-running");
    await onScreen();
    await page.getByRole("tab", { name: "Workflow" }).click();
    await page.getByRole("tab", { name: view }).click();
    if (view === "Canvas") await expect.poll(() => document.querySelectorAll(".react-flow__edge").length, { timeout: 4_000 }).toBeGreaterThan(0);
    await pressed("Add a step after Implement");
  });
}

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
  test(`the menu at the gate is over the next node: ${name}`, RETRIED, async () => {
    await userEvent.unhover(document.body);
    mount("proto/feature-at-approval");
    await drawn();
    await pressed(name);
  });
}
