// The stacked merge line on All: scrolling it stopped halfway between two repositories' rows, a grey scrollbar at a row's end
// (the owner, 23 Oct 2026).

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

/** Chromium's thin classic scrollbar on macOS, with a mouse attached. Headless hides scrollbars, so the row is measured as if one were there. */
const THIN_BAR = 11;

test("on All each stacked repository row fills one snap step and never scrolls down on its own", async () => {
  mount("cockpit-ship-flightdeck");
  await onScreen();
  await expect.element(page.getByRole("region", { name: "Merge queue" })).toBeVisible();
  const stack = document.querySelector<HTMLElement>(".deck-queue__stack");
  expect(stack, "on All with two repositories the line is a stack").not.toBeNull();
  const rows = [...stack!.querySelectorAll<HTMLElement>(":scope > .deck-chips")];
  expect(rows.length).toBeGreaterThan(1);
  expect(stack!.querySelector(".deck-queue__track"), "a row carries the forge's queue track").not.toBeNull();

  for (const row of rows) {
    const name = row.getAttribute("aria-label");
    const style = getComputedStyle(row);
    // A classic scrollbar under the row takes height from it; the row has to hold the track above that scrollbar.
    const bar = Math.max(THIN_BAR, row.offsetHeight - row.clientHeight);
    const room = row.offsetHeight - bar - parseFloat(style.paddingTop) - parseFloat(style.paddingBottom);
    const tallest = Math.max(0, ...[...row.children].map((one) => (one as HTMLElement).offsetHeight));
    expect(["auto", "scroll"], `${name} is no vertical scroller`).not.toContain(style.overflowY);
    expect(row.scrollHeight, `${name} has nothing to scroll down to (bar ${bar}px)`).toBeLessThanOrEqual(row.clientHeight);
    expect(tallest, `${name}'s tallest item fits the room left above its scrollbar (bar ${bar}px)`).toBeLessThanOrEqual(room);
    expect(row.offsetHeight, `${name} is one snap step tall`).toBe(stack!.clientHeight);
  }
});
