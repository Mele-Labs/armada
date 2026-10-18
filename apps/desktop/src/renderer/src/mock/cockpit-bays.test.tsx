// The Cockpit's Bays through `App` on a pool with a bay in every state: grouped under a heading for each repository with none in the
// card, and lit with its holder's row in the list from either end. The look is the walk's
// (`walks/bays-look-like-bays.ts`); this holds what the bays do.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const list = () => page.getByRole("region", { name: "Jobs", exact: true });
const bays = () => page.getByRole("region", { name: "Bays" });
const bay = (name: RegExp) => bays().getByRole("button", { name });
const labels = (selector: string) => [...document.querySelectorAll(selector)].map((one) => one.getAttribute("aria-label"));
const lit = (selector: string) => labels(`${selector}[data-hover]`);

async function deck() {
  mount("bays-look-like-bays");
  await onScreen();
  await expect.element(bays()).toBeVisible();
  await expect.poll(() => labels(".deck-bay").length).toBe(9);
  // The browser's pointer stays where the last test left it, over whatever is drawn there now: put it on something that lights nothing.
  await page.getByRole("region", { name: "Flight controls" }).hover();
  await expect.poll(() => lit(".deck-row").length + lit(".deck-bay").length).toBe(0);
}

test("the bays are grouped under a heading for each repository, and the card names none", async () => {
  await deck();
  const groups = [...document.querySelectorAll(".deck-bays .deck-list__band")];
  expect(groups.map((one) => one.querySelector("h3")?.textContent)).toEqual(["armada", "pocket"]);
  // Every bay stands under the heading of the repository its name says, and the pool's slots are all there.
  for (const group of groups) {
    const repo = group.querySelector("h3")!.textContent!;
    const inside = [...group.querySelectorAll(".deck-bay")];
    expect(inside.length).toBeGreaterThan(0);
    for (const one of inside) expect(one.getAttribute("aria-label")).toContain(`, ${repo}, `);
  }
  // No element in a card says the repository on its own: a branch that starts with it is not that.
  for (const card of document.querySelectorAll(".deck-bay")) {
    expect(card.querySelector(".deck-bay__repo")).toBeNull();
    for (const one of card.querySelectorAll("*")) expect(["armada", "pocket"]).not.toContain(one.textContent);
  }
});

test("a bay under the pointer lights its holder's row, and a row lights the bays it holds", async () => {
  await deck();
  expect(lit(".deck-row")).toEqual([]);

  await bay(/^slot-4, armada, Held/).hover();
  await expect.poll(() => lit(".deck-row")).toEqual(["Pin the store clock"]);
  // The bay itself is lit with it, and nothing is selected by it.
  expect(lit(".deck-bay")).toEqual(["slot-4, armada, Held, Pin the store clock"]);
  expect(document.querySelector(".deck-row[data-selected]")).toBeNull();

  await list().getByRole("button", { name: "Pin the store clock", exact: true }).hover();
  await expect.poll(() => lit(".deck-bay")).toEqual(["slot-4, armada, Held, Pin the store clock"]);
  expect(lit(".deck-row")).toEqual(["Pin the store clock"]);
  expect(document.querySelector(".deck-bay[data-selected]")).toBeNull();
});

test("a free bay has no holder, so it lights nothing", async () => {
  await deck();
  await bay(/^slot-5, armada, Free/).hover();
  // Give a wrongly lit row the frame it would take to show.
  await new Promise((resolve) => setTimeout(resolve, 100));
  expect(lit(".deck-row")).toEqual([]);
  expect(lit(".deck-bay")).toEqual([]);
});
