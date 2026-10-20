// An option's preview is drawn under the answers as the pointer passes over it. Passing over an option with none, or one with a
// longer one, must not change the card's height: the answers would move from under the pointer and the preview would flicker.
// The walk `session-question-preview` draws it; this holds that the card keeps one height.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

test("Preview: pointing at each option in turn leaves the card one height, and the last preview stays", async () => {
  mount("session-question-preview");
  await onScreen();
  await toSession(/Pick the checks/);

  const ask = page.getByRole("article", { name: "Waiting on you" });
  const height = () => ask.element().getBoundingClientRect().height;
  const before = height();

  await userEvent.hover(ask.getByRole("radio", { name: "The shared set" }));
  await expect.element(ask.getByRole("region", { name: "Preview" })).toBeVisible();
  expect(height()).toBe(before);

  await userEvent.hover(ask.getByRole("radio", { name: "None yet" }));
  await expect.element(ask.getByRole("region", { name: "Preview" })).toBeVisible();
  expect(height()).toBe(before);

  await userEvent.hover(ask.getByRole("radio", { name: "Per service" }));
  expect(height()).toBe(before);
});
