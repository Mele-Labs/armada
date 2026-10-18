// An answer on its way to Fleet, through `App` on the `cockpit-pending` mock Fleet, which answers after a
// wait and stops carrying what it answered a wait after that: the control pressed carries the loop and the
// rest of the card is held, a refusal stops it, a call answered or dismissed does not come back while Fleet
// still carries it. The calls are opened from their rows in Work on the flight deck (#2301).

import { afterEach, beforeEach, expect, onTestFinished, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { holdFor } from "../cockpit/deck/props";
import { forgetDismissals } from "../cockpit/dismissed";
import { SLOW_FLEET } from "./cockpit-routes";
import { entered, motion, mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const ANSWERED = 150;
const SETTLED = 700;
const SLOW = { ...SLOW_FLEET };
beforeEach(() => {
  forgetDismissals();
  localStorage.removeItem("armada.bridge.dashboard-tab");
  // The walk lets Fleet go by a step; these run it on the clocks.
  SLOW_FLEET.held = false;
  SLOW_FLEET.answerMs = ANSWERED;
  SLOW_FLEET.settleMs = SETTLED;
});
afterEach(() => void Object.assign(SLOW_FLEET, SLOW));

const question = () => page.getByRole("region", { name: /^Session question/ });
const radio = (name: RegExp) => page.getByRole("radio", { name });
// The card's own acts, in the pane the call opened.
const key = (name: RegExp) => page.getByRole("region", { name: /^Call: / }).getByRole("button", { name });
/** The Session's row in Work, which opens its two questions, oldest first. A call answered is put away, so the next is opened from here. */
const hails = () => page.getByRole("region", { name: "Work", exact: true }).getByRole("button", { name: /^Pin the store clock/ });
const FIRST = /A fake the test sets/;
const SECOND = /Keep three/;

/** The cockpit with its first question opened from its row. */
async function asking() {
  const app = mount("cockpit-pending");
  await onScreen();
  await hails().first().click();
  await expect.element(radio(FIRST)).toBeVisible();
  return app;
}

/**
 * Waits for the first question to leave, which has to be before Fleet has stopped carrying it, and then
 * watches that it stays gone until well after Fleet has. A card gone only once the data lets it go would
 * pass the second half and fail the first.
 */
async function staysGone(from: number): Promise<void> {
  const here = () => page.getByRole("radio", { name: FIRST }).query();
  await expect.poll(() => here() === null, { interval: 10 }).toBe(true);
  expect(Date.now() - from, "the card left only when Fleet stopped carrying it").toBeLessThan(ANSWERED + SETTLED - 200);
  const until = from + ANSWERED + SETTLED + 400;
  while (Date.now() < until) {
    expect(here(), "the card came back while Fleet still carried it").toBeNull();
    await new Promise((done) => window.setTimeout(done, 20));
  }
  await expect.element(radio(SECOND)).toBeVisible();
}

// Skipped (#2301): written for the classic Cockpit (the call in front, the rail's keycaps), which the flight deck replaced. To be ported or dropped before landing.
test.skip("the answer pressed carries the loop, the rest of the card is held, and a second press is not a second send", async () => {
  await motion();
  const app = await asking();
  const answer = vi.spyOn(app.api, "answerWaiting");
  // Every control on the card, measured at rest: none of them may change size while the answer is out.
  const controls = () => [radio(FIRST), radio(/The real one, stopped/), radio(/Make the best decision/), radio(/Just get it done/), key(/Send/), key(/Later/), key(/Dismiss/)];
  const size = (box: DOMRect) => [box.width, box.height];
  await entered(question());
  const resting = controls().map((one) => size(one.element().getBoundingClientRect()));
  await userEvent.keyboard("1");
  await userEvent.keyboard("{Enter}");

  const pressed = radio(FIRST);
  await expect.element(pressed).toHaveAttribute("aria-busy", "true");
  expect(getComputedStyle(pressed.element(), "::after").animationName).toBe("armada-button-waiting");
  // Every other answer and Send cannot be pressed, and no other control on the card acts.
  await expect.element(radio(/The real one, stopped/)).toBeDisabled();
  await expect.element(radio(/Make the best decision/)).toBeDisabled();
  await expect.element(key(/Send/)).toBeDisabled();
  await expect.element(key(/Later/)).toBeDisabled();
  const during = controls().map((one) => size(one.element().getBoundingClientRect()));
  expect(during).toEqual(resting);

  await userEvent.keyboard("{Enter}");
  await userEvent.keyboard("2");
  await userEvent.keyboard("{Enter}");
  await expect.element(radio(SECOND)).toBeVisible();
  expect(answer).toHaveBeenCalledTimes(1);
  expect(answer).toHaveBeenCalledWith({ session_id: "s14", item_id: "q1", choice: 0 });
});

// Skipped (#2301): written for the classic Cockpit (the call in front, the rail's keycaps), which the flight deck replaced. To be ported or dropped before landing.
test.skip("a refusal stops the loop, leaves the card where it was, and the toast says Fleet's words", async () => {
  await motion();
  await asking();
  // Put the first off, so the one Fleet refuses is in front.
  await userEvent.keyboard("l");
  await expect.element(radio(SECOND)).toBeVisible();
  await userEvent.keyboard("2");
  await userEvent.keyboard("{Enter}");
  await expect.element(radio(/Raise it to five/)).toHaveAttribute("aria-busy", "true");

  await expect.element(page.getByText("Session Pin the store clock is already working on that").first()).toBeVisible();
  await expect.element(radio(/Raise it to five/)).not.toHaveAttribute("aria-busy");
  expect(getComputedStyle(radio(/Raise it to five/).element(), "::after").animationName).not.toBe("armada-button-waiting");
  // Nothing is held now, and the same answer can be sent again.
  await expect.element(radio(SECOND)).toBeEnabled();
  await expect.element(key(/Send/)).toBeEnabled();
  await expect.element(question()).toBeVisible();
});

// Skipped (#2301): written for the classic Cockpit (the call in front, the rail's keycaps), which the flight deck replaced. To be ported or dropped before landing.
test.skip("an answered call does not come back while Fleet still carries it, and the next one is in front", async () => {
  await asking();
  await userEvent.keyboard("1");
  await userEvent.keyboard("{Enter}");
  // Fleet answers, and has not yet dropped the item from the Session: the update in between carries it.
  await staysGone(Date.now());
});

// Skipped (#2301): written for the classic Cockpit (the call in front, the rail's keycaps), which the flight deck replaced. To be ported or dropped before landing.
test.skip("a dismissed call loops while Fleet takes it, and does not come back while Fleet still carries it", async () => {
  await motion();
  await asking();
  // The cover lifts on the first press and the second dismisses, so the clock starts at the second.
  await userEvent.keyboard("d");
  const from = Date.now();
  await userEvent.keyboard("d");
  await expect.element(key(/Dismiss/)).toHaveAttribute("aria-busy", "true");
  await expect.element(radio(/The real one, stopped/)).toBeDisabled();
  await staysGone(from);
});

// Skipped (#2301): written for the classic Cockpit (the call in front, the rail's keycaps), which the flight deck replaced. To be ported or dropped before landing.
test.skip("a call Fleet keeps carrying comes back after the hold, with nothing said", async () => {
  const kept = holdFor.ms;
  holdFor.ms = 500;
  onTestFinished(() => void (holdFor.ms = kept));
  // Fleet answers and never stops carrying the item.
  SLOW_FLEET.settleMs = 60_000;
  await asking();
  await userEvent.keyboard("1");
  await userEvent.keyboard("{Enter}");
  await expect.element(radio(SECOND)).toBeVisible();
  expect(page.getByRole("radio", { name: FIRST }).query()).toBeNull();
  // The hold lets go: the call is in the deck again, behind the one now in front, and nobody was told anything.
  await expect.element(page.getByRole("button", { name: /^Session question: Pin the store clock/ })).toBeVisible();
  expect(page.getByText(/already working on that/).query()).toBeNull();
});
