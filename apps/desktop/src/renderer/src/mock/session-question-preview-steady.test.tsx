// An option's preview is a card drawn beside it as the pointer passes over it, out of the question's own box. Passing over
// an option with none, or one with a longer one, must not change that box's height: the answers would move from under the pointer.
// The walk `session-question-preview` draws it; this holds that the box keeps one height and the card goes with the pointer.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

test("Preview: pointing at each option in turn leaves the question one height, and the card goes with the pointer", async () => {
  mount("session-question-preview");
  await onScreen();
  await toSession(/Pick the checks/);

  const ask = page.getByRole("article", { name: "Waiting on you" });
  const height = () => ask.element().getBoundingClientRect().height;
  const before = height();

  await userEvent.hover(ask.getByRole("radio", { name: "The shared set" }));
  await expect.element(page.getByRole("region", { name: "Preview" })).toBeVisible();
  expect(height()).toBe(before);

  await userEvent.hover(ask.getByRole("radio", { name: "None yet" }));
  await expect.element(page.getByRole("region", { name: "Preview" })).not.toBeInTheDocument();
  expect(height()).toBe(before);

  await userEvent.hover(ask.getByRole("radio", { name: "Per service" }));
  await expect.element(page.getByRole("region", { name: "Preview" })).toBeVisible();
  // Its longest line is drawn whole: the card is as wide as the text, not cut at the panel's edge.
  const card = page.getByRole("region", { name: "Preview" }).element();
  expect(card.scrollWidth).toBeLessThanOrEqual(card.clientWidth);
  expect(height()).toBe(before);
});
