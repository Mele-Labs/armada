// The deck's key caps are drawn only while ⌘ is held: the root carries `data-reveal` for as long, and the caps in the deck's bands and on the Flight
// control's tiles are not drawn the rest of the time. Letting go of ⌘, or the window losing focus, hides them again. Through `App` on the flight deck's ship.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const list = () => page.getByRole("region", { name: "Jobs", exact: true });
/** The caps of the deck's bands and tiles, as the stylesheet names where they hide. */
const caps = () => [...document.querySelectorAll<HTMLElement>(".deck :is(.deck-band, .deck-controls, .deck-flight) .armada-kbd")];
const drawn = (cap: HTMLElement) => getComputedStyle(cap).display !== "none";
const revealed = () => "reveal" in document.documentElement.dataset;

async function openJob() {
  mount("cockpit-ship-flightdeck");
  await onScreen();
  await expect.element(list()).toBeVisible();
  await list().getByRole("button", { name: "Cache key", exact: true }).click();
  await expect.element(page.getByRole("region", { name: "Job: Cache key" })).toBeVisible();
  // A cap on a tile and one in a band, so both places are held.
  await expect.poll(() => document.querySelectorAll(".deck-flight .deck-tile .armada-kbd").length).toBeGreaterThan(0);
  expect(document.querySelectorAll(".deck-band .armada-kbd").length).toBeGreaterThan(0);
}

test("the deck's key caps are not drawn until ⌘ is held, then are, and are not again when it is let go", async () => {
  await openJob();
  expect(revealed()).toBe(false);
  expect(caps().filter(drawn)).toHaveLength(0);

  await userEvent.keyboard("{Meta>}");
  try {
    await expect.poll(revealed).toBe(true);
    expect(caps().filter((cap) => !drawn(cap))).toHaveLength(0);
  } finally {
    await userEvent.keyboard("{/Meta}");
  }
  await expect.poll(revealed).toBe(false);
  expect(caps().filter(drawn)).toHaveLength(0);
});

test("the window losing focus while ⌘ is held hides the caps again", async () => {
  await openJob();
  await userEvent.keyboard("{Meta>}");
  try {
    await expect.poll(revealed).toBe(true);
    window.dispatchEvent(new Event("blur"));
    await expect.poll(revealed).toBe(false);
    expect(caps().filter(drawn)).toHaveLength(0);
  } finally {
    await userEvent.keyboard("{/Meta}");
  }
});
