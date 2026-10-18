// An Incoming call opens what it is about, never the card. The Items are the ones the Dashboard builds off a Session's
// `waiting_for`: a Session that asks twice raises two, and Incoming draws one call for the Session, the newest. The card is only
// for a call about neither a Job, a Session nor a pull request, and `cockpit-deck.test.tsx` holds that one. #2301.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const list = () => page.getByRole("region", { name: "Jobs", exact: true });
const incoming = () => page.getByRole("region", { name: "Incoming calls" });
const hails = () => [...document.querySelectorAll<HTMLElement>(".deck-incoming .deck-hail")].map((one) => one.getAttribute("aria-label") ?? "");
/** The call card on the stage. A Session's own pane draws a strip for its call, which is not it. */
const card = () => document.querySelector('.deck-stage > section.deck-pane[aria-label^="Call: "]');

async function deck() {
  mount("cockpit-call-owner");
  await onScreen();
  await expect.element(list()).toBeVisible();
  await expect.poll(() => hails().length).toBe(2);
}

test("an Incoming call from a Session opens that Session on the stage, not the card", async () => {
  await deck();
  // The Session asked twice, and is one call.
  expect(hails().filter((one) => one === "Job question: Session / Job Progress").length).toBe(1);
  await incoming().getByRole("button", { name: "Job question: Session / Job Progress" }).click();
  await expect.element(page.getByRole("region", { name: "Job: Session / Job Progress" })).toBeVisible();
  expect(card()).toBeNull();
  // Seen, it still needs the person, so its call stays.
  expect(hails().filter((one) => one === "Job question: Session / Job Progress").length).toBe(1);
});

test("a call whose Session the deck does not list opens the Session's page, not the card", async () => {
  await deck();
  await incoming().getByRole("button", { name: "Job question: Helm · armada" }).click();
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
