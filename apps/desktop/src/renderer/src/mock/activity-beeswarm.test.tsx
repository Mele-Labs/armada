// Activity's HUD as a beeswarm, through `App` on a ship with an hour of events: hovering a dot opens a card that
// names its events, and pressing the dot picks its event, which rings that dot and no other. The owner's pick of 16 Oct 2026.

import { expect, test } from "vitest";
import { userEvent, page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const dots = () => [
  ...document.querySelectorAll<HTMLElement>('[data-hud="activity"] .armada-tl__dot'),
];
const card = () =>
  document.querySelector(
    '[data-hud="activity"] .armada-tooltip__bubble:not([hidden]) .armada-tl__hour',
  )?.textContent ?? "";

async function hud() {
  mount("activity-beeswarm");
  await onScreen();
  await page
    .getByRole("button", {
      name: "Activity",
      exact: true,
    })
    .click();
  await expect.poll(() => dots().length).toBeGreaterThan(20);
}

test("a dot's card names its events, and pressing it picks its event and rings it", async () => {
  await hud();
  // The newest dot is the last, as they are drawn oldest first. Its name is its Job or Session and what it did.
  const dot = dots().at(-1)!;
  const [who, ...rest] = dot.getAttribute("aria-label")!.split(": ");
  const said = rest.join(": ");
  expect(dots().filter((one) => one.hasAttribute("data-lit"))).toHaveLength(0);

  await userEvent.hover(page.elementLocator(dot));
  await expect.poll(card).toContain(who!);
  expect(card()).toContain(said);

  await userEvent.click(page.elementLocator(dot));
  await expect
    .poll(() => dots().filter((one) => one.hasAttribute("data-lit")))
    .toEqual([
      dot,
    ]);
});

test("the arrow keys step along the dots, and no dot carries its place as an attribute that every event would rewrite", async () => {
  await hud();
  expect(dots().filter((one) => one.hasAttribute("data-i"))).toHaveLength(0);
  const last = dots().at(-1)!;
  last.focus();
  await userEvent.keyboard("{ArrowLeft}");
  expect(document.activeElement).toBe(dots().at(-2));
  await userEvent.keyboard("{ArrowRight}");
  expect(document.activeElement).toBe(last);
});
