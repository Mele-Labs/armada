// The deck's merge line: one row that scrolls across, the forge's queue framed in its order with each place, and the other pull
// requests past a rule (the owner, 15 Oct 2026).

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

test("the queue is framed in its order, the rest follow, and the line scrolls rather than wraps", async () => {
  mount("cockpit-ship-flightdeck");
  await onScreen();
  await expect.element(page.getByRole("region", { name: "Merge queue" })).toBeVisible();
  const chips = document.querySelector<HTMLElement>(".deck-queue .deck-chips")!;
  expect(getComputedStyle(chips).flexWrap).toBe("nowrap");
  expect(getComputedStyle(chips).overflowX).toBe("auto");

  const track = document.querySelector<HTMLElement>(".deck-queue__track");
  expect(track, "a queued pull request is drawn in the track").not.toBeNull();
  const places = [...track!.querySelectorAll(".deck-chip__place")].map((one) => Number(one.textContent));
  expect(places.length).toBeGreaterThan(0);
  expect(places).toEqual([...places].sort((a, b) => a - b));
  // Nothing outside the track carries a place.
  expect([...chips.querySelectorAll(".deck-chip__place")].length).toBe(places.length);
});
