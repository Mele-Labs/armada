// The flight deck's Activity as a beeswarm, through `App` on a ship with an hour of events: hovering a dot opens a card that
// names its events, and pressing the dot picks its event, which rings that dot and no other. The owner's pick of 16 Oct 2026.

import { expect, test } from "vitest";
import { userEvent, page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const dots = () => [...document.querySelectorAll<HTMLElement>(".deck-activity .armada-tl__dot")];
const card = () => document.querySelector(".deck-activity .armada-tooltip__bubble:not([hidden]) .armada-tl__hour")?.textContent ?? "";

test("a dot's card names its events, and pressing it picks its event and rings it", async () => {
  mount("activity-beeswarm");
  await onScreen();
  await expect.poll(() => dots().length).toBeGreaterThan(20);
  // The newest dot is the last, as they are drawn oldest first. Its name is its Job or Session and what it did.
  const dot = dots().at(-1)!;
  const [who, ...rest] = dot.getAttribute("aria-label")!.split(": ");
  const said = rest.join(": ");
  expect(dots().filter((one) => one.hasAttribute("data-lit"))).toHaveLength(0);

  await userEvent.hover(page.elementLocator(dot));
  await expect.poll(card).toContain(who!);
  expect(card()).toContain(said);

  await userEvent.click(page.elementLocator(dot));
  await expect.poll(() => dots().filter((one) => one.hasAttribute("data-lit"))).toEqual([dot]);
});
