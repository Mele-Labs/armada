// A red main's light on the Cockpit's merge queue opens a card with the Merge line's panel for the red: what failed, who holds it,
// and the same acts to hand it on. A green main has no card. The moments are `main-red-hub`'s: 2 is green, 3 is red with nobody on
// it.

import { repository } from "@armada/screens/src/fixtures/repository";
import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mainRed } from "./main-red-hub";
import { scenarioNamed, type Scenario } from "./scenario";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

function standingAt(moment: number): Scenario {
  const { later } = mainRed(repository().root);
  const base = scenarioNamed("main-red-hub")!;
  return {
    ...base,
    state: { ...base.state, ...later[moment - 1]! },
    later: [],
  };
}

const light = () => page.getByRole("button", { name: /^Main is red in/ });
/** The inspect card: a tooltip while only hovered or focused, a dialog once a press has pinned it. */
const card = () => page.getByRole("dialog", { name: /^Main is red in/ });
const hovered = () => page.getByRole("tooltip");

async function deck(scenario: Scenario) {
  mount(scenario);
  await onScreen();
  await expect.element(page.getByRole("region", { name: "Jobs", exact: true })).toBeVisible();
}

test("hovering the red light opens the card with the way to hand the red to a Session", async () => {
  await deck(standingAt(3));
  expect(hovered().query()).toBeNull();
  await light().hover();
  await expect.element(hovered()).toBeVisible();
  await expect.element(hovered().getByText("desktop_test")).toBeVisible();
  await expect.element(hovered().getByRole("button", { name: "Dispatch a new Job" })).toBeVisible();
  expect(hovered().getByRole("button", { name: "Dispatch a new Job" }).query()).toBeNull();
});

test("the card's Dispatch a new Job drafts a Job holding the red and opens it, as the Merge line does", async () => {
  await deck(standingAt(3));
  await light().click();
  await card().getByRole("button", { name: "Dispatch a new Job" }).click();
  const job = page.getByRole("region", { name: /^Job: / });
  await expect.element(job).toBeVisible();
  await expect.element(job).toHaveTextContent("desktop_test fails on main.");
  expect(page.getByRole("region", { name: "Dispatch", exact: true }).query()).toBeNull();
  expect(card().query()).toBeNull();
});

test("a press pins the card, and Esc puts it away", async () => {
  await deck(standingAt(3));
  await light().click();
  await expect.element(card()).toBeVisible();
  // Off the light, a pinned card stays.
  await page.getByRole("region", { name: "Jobs", exact: true }).hover();
  await new Promise((done) => window.setTimeout(done, 400));
  await expect.element(card()).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => card().query()).toBeNull();
});

test("the light is reached by keyboard and opens the card on focus", async () => {
  await deck(standingAt(3));
  (light().element() as HTMLElement).focus();
  await expect.element(hovered()).toBeVisible();
});

test("a green main has no card", async () => {
  await deck(standingAt(2));
  expect(light().query()).toBeNull();
  const green = page.getByRole("img", { name: "Main is green" });
  await green.hover();
  await new Promise((done) => window.setTimeout(done, 200));
  expect(page.getByRole("dialog", { name: /^Main is/ }).query()).toBeNull();
});
