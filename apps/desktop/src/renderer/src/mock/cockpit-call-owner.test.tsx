// An Incoming call opens what it is about, never the card. The Items are the ones the Dashboard builds off a Session's
// `waiting_for`, so a Session that asks twice has two calls and one of them is not the key on its subject. The card is only
// for a call about neither a Job, a Session nor a pull request, and `cockpit-deck.test.tsx` holds that one. #2301.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const list = () => page.getByRole("region", { name: "Jobs and Sessions" });
const incoming = () => page.getByRole("region", { name: "Incoming calls" });
const hails = () => [...document.querySelectorAll<HTMLElement>(".deck-incoming .deck-hail")].map((one) => one.getAttribute("aria-label") ?? "");
/** The call card on the stage. A Session's own pane draws a strip for its call, which is not it. */
const card = () => document.querySelector('.deck-stage > section.deck-pane[aria-label^="Call: "]');

async function deck() {
  mount("cockpit-call-owner");
  await onScreen();
  await expect.element(list()).toBeVisible();
  await expect.poll(() => hails().length).toBe(3);
}

test("an Incoming call from a Session opens that Session on the stage, not the card", async () => {
  await deck();
  // Both of the Session's questions, the one its row carries and the one it does not.
  for (const at of [0, 1]) {
    const mine = hails().filter((one) => one === "Session question: Session / Job Progress");
    expect(mine.length).toBe(2 - at);
    await incoming().getByRole("button", { name: "Session question: Session / Job Progress" }).first().click();
    await expect.element(page.getByRole("region", { name: "Session: Session / Job Progress" })).toBeVisible();
    expect(card()).toBeNull();
  }
});

test("a call whose Session the deck does not list opens the Session's page, not the card", async () => {
  await deck();
  await incoming().getByRole("button", { name: "Session question: Helm · armada" }).click();
  // Helm's Session is the dock, which is where a link to it goes.
  await expect.element(page.getByRole("complementary", { name: "Helm" })).toBeVisible();
  expect(card()).toBeNull();
});

// What Sleep decided is about no Job, Session or pull request, so it is the one call the card is for.
test("a call about nothing opens the card", async () => {
  mount("sleep-cockpit");
  await onScreen();
  await incoming().getByRole("button", { name: "Decided while you slept: Order the lunch" }).click();
  await expect.element(page.getByRole("region", { name: "Call: Order the lunch" })).toBeVisible();
  expect(card()).not.toBeNull();
});
