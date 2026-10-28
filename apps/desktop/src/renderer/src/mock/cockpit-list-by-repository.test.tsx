// The flight deck's Sessions list through `App`: on All with two repositories served the list is headed by
// repository in the listing's order, each heading holding its own rows; one repository picked draws no heading; and the
// cursor walks the rows in the order they are drawn. The walk `cockpit-list-by-repository` draws it.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const list = () => page.getByRole("region", { name: "Jobs", exact: true });
const headings = () => [...document.querySelectorAll(".deck-list__head")].map((one) => one.textContent);
const drawn = (heading: string) => {
  const band = [...document.querySelectorAll(".deck-list__band")].find((one) => one.querySelector(".deck-list__head")?.textContent === heading);
  return [...(band?.querySelectorAll("[data-row]") ?? [])].map((one) => one.getAttribute("aria-label"));
};
const keys = () => [...document.querySelectorAll(".deck-list [data-row]")].map((one) => one.getAttribute("data-row")!);
const rowOf = (key: string) => document.querySelector<HTMLElement>(`.deck-list [data-row="${CSS.escape(key)}"]`)!;

async function onAll() {
  mount("cockpit-list-by-repository");
  await onScreen();
  await expect.element(list()).toBeVisible();
  await expect.poll(() => headings().length).toBe(2);
}

test("All heads the list by repository, in the listing's order, each with its own rows", async () => {
  await onAll();
  expect(headings()).toEqual(["armada", "storefront"]);
  expect([...drawn("armada")].sort()).toEqual(["Draft the migration notes", "Fix the flaky store test", "Prune the stale branches"]);
  expect([...drawn("storefront")].sort()).toEqual(["Lay out the invoice PDF", "Round the cart totals"]);
});

test("the cursor steps from the last row of one group to the first of the next", async () => {
  await onAll();
  const rows = keys();
  const last = drawn("armada").length - 1;
  rowOf(rows[last]!).focus();
  await userEvent.keyboard("j");
  await expect.poll(() => document.activeElement?.getAttribute("data-row")).toBe(rows[last + 1]);
  expect(rowOf(rows[last + 1]!).getAttribute("aria-label")).toBe(drawn("storefront")[0]);
});

test("one repository picked draws the list with no heading and only its Sessions", async () => {
  await onAll();
  await userEvent.click(page.getByRole("button", { name: "Repository", exact: true }));
  await userEvent.click(page.getByRole("menuitem", { name: "storefront" }));
  await expect.element(list().getByRole("button", { name: "Round the cart totals" })).toBeVisible();
  expect(headings()).toEqual([]);
  await expect.element(list().getByRole("button", { name: "Prune the stale branches" })).not.toBeInTheDocument();
  await expect.element(list().getByRole("button", { name: "Lay out the invoice PDF" })).toBeVisible();
  // The rows sit straight in the one list, as they did before.
  expect(document.querySelectorAll(".deck-list__rows > .deck-list__item")).toHaveLength(keys().length);
});
