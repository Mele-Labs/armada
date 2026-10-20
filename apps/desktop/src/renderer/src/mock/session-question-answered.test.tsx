// A question answered in the Needs you panel's detail leaves its number in a circle beside Back, named by its header and what was chosen; a press on
// it brings that question back. The walk `session-question` draws the deck; this holds where the mark sits.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

test("Answered: its number in a circle beside Back names the answer, and reopens its question", async () => {
  mount("session-question");
  await onScreen();
  await toSession(/Order the lunch/);

  const ask = page.getByRole("article", { name: "Waiting on you" });
  await userEvent.click(ask.getByRole("radio", { name: "Large" }));
  await expect.element(ask.getByText("Which toppings?")).toBeVisible();

  const mark = ask.getByRole("button", { name: "Size: Large" });
  await expect.element(mark).toBeVisible();
  expect(mark.element().textContent?.trim()).toBe("1");
  const circle = mark.element().querySelector(".armada-question-deck__number")!.getBoundingClientRect();
  expect(circle.width).toBeGreaterThan(8);
  expect(circle.width).toBeCloseTo(circle.height, 0);
  const back = page.getByRole("region", { name: "Needs you" }).getByRole("button", { name: "Back" }).element().getBoundingClientRect();
  const at = mark.element().getBoundingClientRect();
  expect(Math.abs(at.top - back.top)).toBeLessThan(8);
  expect(at.left).toBeGreaterThan(back.right);

  await userEvent.click(mark);
  await expect.element(ask.getByText("Which size?")).toBeVisible();
});
