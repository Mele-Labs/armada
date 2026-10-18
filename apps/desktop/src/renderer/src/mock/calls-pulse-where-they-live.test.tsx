// A call no Job's row holds pulses the place that shows it, through `App` on `calls-pulse-where-they-live`: main red marks main's
// light in the merge queue, a failed pull request marks its chip, a pull request that passed stays quiet, and c reaches main's light
// when no row needs him. The walk of the same name draws it; this holds that the presses reach the same code.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const queue = () => page.getByRole("region", { name: "Merge queue" });
const main = () => queue().getByRole("button", { name: /^Main is red/ });
const chip = (number: number) => queue().getByRole("button", { name: new RegExp(`^#${number}`) });
const calling = (one: { element: () => Element }) => one.element().matches("[data-calling]");

test("main red marks main's light, a failed pull request marks its chip, and a passed one stays quiet", async () => {
  mount("calls-pulse-where-they-live");
  await onScreen();
  await expect.element(main()).toBeVisible();

  expect(calling(main())).toBe(true);
  expect(calling(chip(1896))).toBe(true);
  expect(calling(chip(1892))).toBe(false);
  // No Job's row holds either call, so no row pulses.
  expect(document.querySelector(".deck-row[data-calling], .deck-row[data-needs]")).toBeNull();
});

test("c goes to main's light when no row needs him", async () => {
  mount("calls-pulse-where-they-live");
  await onScreen();
  await expect.element(main()).toBeVisible();
  await userEvent.keyboard("{Escape}");

  await userEvent.keyboard("c");
  await expect.element(main()).toHaveFocus();
});
