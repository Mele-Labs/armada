// A call opens what it is about, never the card. The Items are the ones the Dashboard builds off a Session's
// `waiting_for`: a Session that asks twice raises two, and its row calls once, for the newest. A call with no row is reached by `w`. The card is only
// for a call about neither a Job, a Session nor a pull request, and `cockpit-deck.test.tsx` holds that one. #2301.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const list = () => page.getByRole("region", { name: "Work", exact: true });
/** The rows that call him: pulsing, and the titles they carry. */
const hails = () => [...document.querySelectorAll<HTMLElement>(".deck-row[data-calling]")].map((one) => one.getAttribute("aria-label") ?? "");
/** The call card on the stage. A Session's own pane draws a strip for its call, which is not it. */
const card = () => document.querySelector('.deck-stage > section.deck-pane[aria-label^="Call: "]');

async function deck() {
  mount("cockpit-call-owner");
  await onScreen();
  await expect.element(list()).toBeVisible();
  await expect.poll(() => hails().length).toBe(1);
}

test("a calling row opens its Session on the stage, not the card", async () => {
  await deck();
  // The Session asked twice, and is one calling row.
  expect(hails().filter((one) => one === "Session / Job Progress").length).toBe(1);
  await list().getByRole("button", { name: "Session / Job Progress", exact: true }).click();
  await expect.element(page.getByRole("region", { name: "Session: Session / Job Progress" })).toBeVisible();
  expect(card()).toBeNull();
  // Open, its calls are cards in its thread and its row is quiet.
  expect(hails()).not.toContain("Session / Job Progress");
});

test("a call whose Session the deck does not list opens the Session's page, not the card", async () => {
  await deck();
  // It has no row, so the key that brings the next call forward reaches it: the newest call is Helm's.
  await userEvent.keyboard("w");
  // Helm's Session is the dock, which is where a link to it goes.
  await expect.element(page.getByRole("complementary", { name: "Helm" })).toBeVisible();
  expect(card()).toBeNull();
});

// What Sleep decided is about no Job, Session or pull request, so it is the one call the card is for.
test("a call about nothing opens the card", async () => {
  mount("sleep-cockpit");
  await onScreen();
  // The first key turns Sleep off, and the decisions arrive as calls; the next brings one forward.
  await userEvent.keyboard("w");
  await userEvent.keyboard("w");
  await expect.element(page.getByRole("region", { name: /^Call: / })).toBeVisible();
  expect(card()).not.toBeNull();
});
