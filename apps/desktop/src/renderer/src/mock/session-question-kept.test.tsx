// A question answered in the Needs you panel is kept like a message not yet sent: leave the Session and come back, and the answers
// given and the question in front are as he left them. Sending clears what was kept.
// The walk `session-question-many` draws it; this holds that the presses reach the same code.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

const kept = () => Object.keys(localStorage).filter((key) => key.startsWith("armada:kept:question:"));

test("Kept: answers and the front question survive leaving the Session, and sending clears them", async () => {
  mount("session-question");
  await onScreen();
  await toSession(/Order the lunch/);

  const ask = page.getByRole("article", { name: "Waiting on you" });
  await userEvent.click(ask.getByRole("radio", { name: "Large" }));
  await expect.element(ask.getByText("Which toppings?")).toBeVisible();
  await userEvent.click(ask.getByRole("checkbox", { name: "Cheese" }));

  await toSession(/Name the branch/);
  await toSession(/Order the lunch/);

  await expect.element(ask.getByText("Which toppings?")).toBeVisible();
  await expect.element(ask.getByRole("checkbox", { name: "Cheese" })).toBeChecked();
  await expect.element(ask.getByRole("button", { name: "Size: Large" })).toBeVisible();
  expect(kept()).toHaveLength(1);

  await userEvent.click(ask.getByRole("button", { name: "Answer" }));
  await expect.element(page.getByRole("region", { name: "Thread" }).getByText(/Going with Large/)).toBeVisible();
  expect(kept()).toHaveLength(0);
});
