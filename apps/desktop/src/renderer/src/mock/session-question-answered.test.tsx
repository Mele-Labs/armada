// The Needs you panel's detail has a stepper beside Back: a node per question, the answered ones named by their header and what was chosen,
// the one in front marked, any of them a press away. The walk `session-question-many` draws it; this holds where the stepper sits.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

test("Stepper: a node per question beside Back, answered ones named, any one reopens its question", async () => {
  mount("session-question");
  await onScreen();
  await toSession(/Order the lunch/);

  const ask = page.getByRole("article", { name: "Waiting on you" });
  const steps = ask.getByRole("group", { name: "Questions" });
  await expect.element(steps).toBeVisible();
  expect(steps.getByRole("button").elements()).toHaveLength(2);
  await expect.element(steps.getByRole("button", { name: "Size", exact: true })).toHaveAttribute("aria-current", "step");

  await userEvent.click(ask.getByRole("radio", { name: "Large" }));
  await expect.element(ask.getByText("Which toppings?")).toBeVisible();
  const done = steps.getByRole("button", { name: "Size: Large" });
  await expect.element(done).toBeVisible();
  await expect.element(steps.getByRole("button", { name: "Toppings", exact: true })).toHaveAttribute("aria-current", "step");

  const back = page.getByRole("region", { name: "Needs you" }).getByRole("button", { name: "Back" }).element().getBoundingClientRect();
  const at = steps.element().getBoundingClientRect();
  expect(Math.abs(at.top - back.top)).toBeLessThan(8);
  expect(at.left).toBeGreaterThan(back.right);

  await userEvent.click(done);
  await expect.element(ask.getByText("Which size?")).toBeVisible();
  // Any node reaches its question, answered or not.
  await userEvent.click(steps.getByRole("button", { name: "Toppings", exact: true }));
  await expect.element(ask.getByText("Which toppings?")).toBeVisible();
});
