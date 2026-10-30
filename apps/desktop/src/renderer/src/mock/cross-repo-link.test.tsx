// The Cockpit's list through `App`, on All with two repositories served: a Job that started one in its own repository and one in
// another. The first hangs under it with the line; the second stands in its own repository's group with no indent and no line, its
// link glyph naming its starter, and moves among that group's rows. The walk `crossRepoLink` draws it.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const list = () => page.getByRole("region", { name: "Jobs", exact: true });
const itemOf = (title: string) => document.querySelector<HTMLElement>(`.deck-list [aria-label="${CSS.escape(title)}"]`)!.closest<HTMLElement>(".deck-list__item")!;
const drawn = (heading: string) => {
  const band = [...document.querySelectorAll(".deck-list__band")].find((one) => one.querySelector(".deck-list__head")?.textContent === heading);
  return [...(band?.querySelectorAll("[data-row]") ?? [])].map((one) => one.getAttribute("aria-label"));
};

async function onAll() {
  mount("cross-repo-link");
  await onScreen();
  await expect.element(list().getByRole("button", { name: "Round the cart totals" })).toBeVisible();
}

test("a Job started from another repository stands in its own group with no line, its glyph naming its starter", async () => {
  await onAll();
  expect(drawn("storefront")).toEqual(["Lay out the invoice PDF", "Round the cart totals"]);
  const away = itemOf("Round the cart totals");
  expect(away.querySelector(".deck-row")!.hasAttribute("data-linked")).toBe(false);
  expect(away.querySelectorAll(".deck-list__rail")).toHaveLength(0);
  expect(away.querySelector(".deck-row__link")?.getAttribute("aria-label")).toBe("Linked Job, started by Plan the checkout rework in armada");
  expect(away.getAttribute("data-movable")).toBe("storefront");
});

test("its sibling in the starter's own repository hangs under it with no line running on to the one away", async () => {
  await onAll();
  expect(drawn("armada")).toEqual(["Plan the checkout rework", "Trim the retry loop", "Draft the migration notes"]);
  const home = itemOf("Trim the retry loop");
  expect(home.querySelector(".deck-row")!.hasAttribute("data-linked")).toBe(true);
  expect(home.querySelectorAll(".deck-list__rail")).toHaveLength(0);
});
