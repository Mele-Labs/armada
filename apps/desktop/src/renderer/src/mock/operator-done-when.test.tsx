// What armada-ehhu.10 is done on, past its walk: the Up next order moves from the board, a raise no Job owns is
// answered from Needs you, and a ruling is corrected from its Log row.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

async function toOperator(): Promise<void> {
  mount("operator-take-board");
  await onScreen();
  await userEvent.keyboard("{Meta>}j{/Meta}");
  await expect.element(page.getByRole("region", { name: "Operator", exact: true })).toBeVisible();
}

const lane = (name: string) => page.getByRole("region", { name, exact: true });

test("⌥↑ on an Up next card moves it above the one before", async () => {
  await toOperator();
  const next = lane("Up next");
  await expect.element(next.getByRole("listitem", { name: "Doctor lists new checks" })).toBeVisible();
  const order = () =>
    next
      .getByRole("listitem")
      .elements()
      .map((one) => one.getAttribute("aria-label"));
  const before = order();
  const at = before.indexOf("Doctor lists new checks");
  expect(at).toBeGreaterThan(0);

  next.getByRole("listitem", { name: "Doctor lists new checks" }).element().focus();
  await userEvent.keyboard("{Alt>}{ArrowUp}{/Alt}");

  await expect.poll(() => order().indexOf("Doctor lists new checks")).toBe(at - 1);
  expect(order()[at]).toBe(before[at - 1]);
});

test("a raise no Job owns shows in Needs you with its ways, and a press settles it", async () => {
  await toOperator();
  const needs = lane("Needs you");
  await expect.element(needs.getByText("desktop_unit failed on main")).toBeVisible();
  await needs.getByRole("button", { name: "Start a fix: desktop_unit failed on main" }).click();

  await expect.poll(() => needs.getByText("desktop_unit failed on main").query()).toBeNull();
  await expect.element(lane("Done").getByText("desktop_unit failed on main")).toBeVisible();
});

test("Overrule on a Log row records the way he picked", async () => {
  await toOperator();
  await page.getByRole("region", { name: "Operator", exact: true }).getByRole("tab", { name: "Log", exact: true }).click();

  await page.getByRole("button", { name: "Overrule: Test renamed in the plan. Keep the old name?" }).click();
  await page.getByRole("menuitem", { name: "Rename it" }).click();

  await expect.element(page.getByText("You changed it to Rename it").first()).toBeVisible();
});
