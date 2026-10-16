// The calls pinned under a Session's thread, through `App` on the `pinned-calls` scenario at a 1512 by 860 window: scrolling the thread
// up shrinks the region to its band, a press on the band opens it over the thread, and a second press puts the thread back where it was.
// Hand-run with the rest of `mock/`.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { s262PinnedCalls } from "./scenarios/pinned-calls";
import { mount, motion, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const thread = () => page.getByRole("region", { name: "Thread", exact: true });
const calls = () => page.getByRole("region", { name: "Pinned calls" });
const band = () => calls().getByRole("button", { name: /^Plan to approve, Question/ });

const scroller = () => thread().element() as HTMLElement;
const stage = () => scroller().parentElement!.getBoundingClientRect();
/** How much of the region is on the stage: its box, less what has slid below the stage's foot. */
const shown = () => Math.round(Math.min(calls().element().getBoundingClientRect().bottom, stage().bottom) - calls().element().getBoundingClientRect().top);
/** The band, its two borders and nothing else. */
const bandOnly = () => Math.round(band().element().getBoundingClientRect().height) + 2;

async function open() {
  await page.viewport(1512, 860);
  mount(s262PinnedCalls);
  await onScreen();
  await userEvent.click(page.getByRole("region", { name: "Jobs and Sessions" }).getByRole("button", { name: "Fold Jobs into Sessions" }));
  await expect.element(calls()).toBeVisible();
  // The thread opens at its newest row, with the region whole.
  await expect.poll(() => scroller().scrollHeight - scroller().clientHeight - scroller().scrollTop).toBeLessThan(2);
}

test("scrolling the thread up leaves the region its band, and the end of the thread has it whole", async () => {
  await open();
  const whole = shown();
  expect(whole).toBeGreaterThan(bandOnly() + 100);
  expect(whole).toBeLessThanOrEqual(Math.ceil(stage().height * 0.4) + bandOnly());

  scroller().scrollTop = 0;
  await expect.poll(shown).toBe(bandOnly());
  await expect.element(band()).toBeVisible();

  scroller().scrollTop = scroller().scrollHeight;
  await expect.poll(shown).toBe(whole);
});

test("the region follows the scroll a pixel for a pixel, and the last row stays on its top edge", async () => {
  await motion();
  await open();
  const whole = shown();
  const end = scroller().scrollHeight - scroller().clientHeight;
  scroller().scrollTop = end - 60;
  await expect.poll(() => Math.abs(whole - shown() - 60)).toBeLessThanOrEqual(1);
  // The thread's own rows are not resized by it.
  expect(scroller().clientHeight).toBeGreaterThan(0);
});

test("a press on the band opens the region over the whole thread", async () => {
  await open();
  scroller().scrollTop = 0;
  await expect.poll(shown).toBe(bandOnly());

  await band().click();
  await expect.element(band()).toHaveAttribute("aria-expanded", "true");
  await expect.poll(() => Math.round(calls().element().getBoundingClientRect().height)).toBe(Math.round(stage().height));
  expect(Math.round(calls().element().getBoundingClientRect().top)).toBe(Math.round(stage().top));
  await expect.element(page.getByRole("article", { name: "Waiting on you" })).toBeVisible();
  await expect.element(page.getByRole("article", { name: "Plan to approve" })).toBeVisible();
});

test("a second press, or Esc, puts the thread back where it was", async () => {
  await open();
  scroller().scrollTop = 300;
  await expect.poll(shown).toBe(bandOnly());
  const left = scroller().scrollTop;

  await band().click();
  await expect.element(band()).toHaveAttribute("aria-expanded", "true");
  expect(scroller().scrollTop).toBe(left);
  await band().click();
  await expect.element(band()).toHaveAttribute("aria-expanded", "false");
  await expect.poll(shown).toBe(bandOnly());
  expect(scroller().scrollTop).toBe(left);

  await band().click();
  await userEvent.keyboard("{Escape}");
  await expect.element(band()).toHaveAttribute("aria-expanded", "false");
  expect(scroller().scrollTop).toBe(left);

  // Enter on the focused band does what a press does.
  band().element().focus();
  await userEvent.keyboard("{Enter}");
  await expect.element(band()).toHaveAttribute("aria-expanded", "true");
});
