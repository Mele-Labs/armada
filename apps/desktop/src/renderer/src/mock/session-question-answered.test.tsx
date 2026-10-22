// The Needs you panel's detail has a stepper beside its buttons: a node per question, the answered ones named by their header and what was chosen,
// the one in front marked, any of them a press away. The walk `session-question-many` draws it; this holds where the stepper sits.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

test("Stepper: a node per question beside the buttons, answered ones named, any one reopens its question", async () => {
  mount("session-question");
  await onScreen();
  await toSession(/Order the lunch/);

  const ask = page.getByRole("article", { name: "Waiting on you" });
  const steps = ask.getByRole("group", { name: "Questions" });
  await expect.element(steps).toBeVisible();
  expect(steps.getByRole("button").elements()).toHaveLength(2);
  await expect.element(steps.getByRole("button", { name: "Size", exact: true })).toHaveAttribute("aria-current", "step");
  // Not on the last question, the button moves on rather than sends.
  const answers = ask.getByRole("group", { name: "Answers" });
  await expect.element(answers.getByRole("button", { name: "Next" })).toBeVisible();
  expect(answers.getByRole("button", { name: "Answer" }).elements()).toHaveLength(0);

  await userEvent.click(ask.getByRole("radio", { name: "Large" }));
  await expect.element(ask.getByText("Which toppings?")).toBeVisible();
  const done = steps.getByRole("button", { name: "Size: Large" });
  await expect.element(done).toBeVisible();
  await expect.element(steps.getByRole("button", { name: "Toppings", exact: true })).toHaveAttribute("aria-current", "step");

  // On the last it sends, and the steps sit in the same row as it.
  await expect.element(answers.getByRole("button", { name: "Answer" })).toBeVisible();
  expect(answers.getByRole("group", { name: "Questions" }).elements()).toHaveLength(1);
  const answer = answers.getByRole("button", { name: "Answer" }).element().getBoundingClientRect();
  const at = steps.element().getBoundingClientRect();
  expect(Math.abs(at.top + at.height / 2 - (answer.top + answer.height / 2))).toBeLessThan(8);

  await userEvent.click(done);
  await expect.element(ask.getByText("Which size?")).toBeVisible();
  // Any node reaches its question, answered or not.
  await userEvent.click(steps.getByRole("button", { name: "Toppings", exact: true }));
  await expect.element(ask.getByText("Which toppings?")).toBeVisible();
});

test("Answer: once every question has an answer it takes the accent fill, not the panel's chip", async () => {
  mount("session-question");
  await onScreen();
  await toSession(/Order the lunch/);

  const ask = page.getByRole("article", { name: "Waiting on you" });
  const fill = (element: Element) => getComputedStyle(element).backgroundColor;
  const accent = document.createElement("span");
  accent.style.background = "var(--accent)";
  document.body.append(accent);
  await userEvent.click(ask.getByRole("radio", { name: "Large" }));
  await userEvent.click(ask.getByRole("checkbox", { name: "Cheese" }));
  const answer = ask.getByRole("button", { name: "Answer" });
  await expect.element(answer).toBeEnabled();
  expect(fill(answer.element())).toBe(fill(accent));
  accent.remove();
});
