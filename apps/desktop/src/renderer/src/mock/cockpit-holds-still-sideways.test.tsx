// A Job's panes reach half a gutter past the deck's edge, and the screen around the deck scrolls. Unclipped, that reach let two fingers slide the whole Cockpit 3px sideways: the
// owner's report of 2 Nov 2026.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

test("the Cockpit has nothing to scroll sideways with a Job's panes open", async () => {
  mount("thread-head-room");
  await onScreen();
  await page.getByRole("button", { name: "Retry the slot lease when two Jobs ask for the same bay at once" }).click();
  await expect.element(page.getByRole("region", { name: "Pane: Thread", exact: true })).toBeVisible();

  const screen = document.querySelector<HTMLElement>(".armada-screen__overview");
  const area = document.querySelector<HTMLElement>(".job-panes__area");
  expect(area!.getBoundingClientRect().right).toBeGreaterThan(screen!.getBoundingClientRect().right);
  expect(screen!.scrollWidth).toBe(screen!.clientWidth);
});
